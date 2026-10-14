//! Foreign-parent embedding hook: GPUI inside a host-owned `HWND`.
//!
//! A VST3-style host owns the parent window and the message loop; GPUI only
//! provides a `WS_CHILD` window plus a synchronous frame entry point. The
//! embedder (toolkit `gpui-win-foreign`) stashes a [`ForeignParentInfo`]
//! through [`stash_pending_foreign`] before `open_window`, and collects the
//! resulting [`ForeignWindow`] through [`take_opened_foreign`] after it.
//! Both handoffs are one-shot thread-locals, consumed on the host UI thread.
//!
//! Contract: toolkit `docs/foreign-parent-editor-embedding.md` §10
//! (Windows row). The app path (`WindowsWindow::new`) is untouched; every
//! foreign-only behavior lives in `WindowsWindow::new_foreign` plus the
//! small branches that route to it.

use std::cell::RefCell;
use std::rc::Rc;

use ::util::ResultExt;
use anyhow::Context as _;
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::*,
};

use crate::*;
use gpui::*;

/// One-shot attach record for a foreign-parent editor window.
///
/// `parent` is the raw host `HWND` value (`HWND.0 as isize`); `width` and
/// `height` are the initial child size in physical pixels; `scale` is the
/// host backing scale used to derive logical bounds.
#[derive(Debug, Clone, Copy)]
pub struct ForeignParentInfo {
    /// Raw host `HWND` value that becomes the `WS_CHILD` parent.
    pub parent: isize,
    /// Initial child width in physical pixels (clamped to `>= 1`).
    pub width: i32,
    /// Initial child height in physical pixels (clamped to `>= 1`).
    pub height: i32,
    /// Host backing scale; must be finite and positive.
    pub scale: f32,
}

thread_local! {
    static PENDING_FOREIGN: RefCell<Option<ForeignParentInfo>> = const { RefCell::new(None) };
    static OPENED_FOREIGN: RefCell<Option<Rc<WindowsWindowInner>>> =
        const { RefCell::new(None) };
}

/// Stash the attach record the next `open_window` consumes.
///
/// Called by the embedder on the host UI thread immediately before
/// `cx.open_window`. Replaces any previous record; a stale record is
/// cleared explicitly with [`clear_pending_foreign`] on failure paths so
/// it can never leak into an unrelated window creation.
pub fn stash_pending_foreign(info: ForeignParentInfo) {
    PENDING_FOREIGN.with(|pending| *pending.borrow_mut() = Some(info));
}

/// Drop a stashed attach record without consuming it.
///
/// Failure-path cleanup for the embedder: call when window creation did
/// not reach `open_window` (or failed there) so the record cannot leak
/// into a later, unrelated creation.
pub fn clear_pending_foreign() {
    PENDING_FOREIGN.with(|pending| *pending.borrow_mut() = None);
}

/// Take the attach record stashed for this `open_window`, if any.
pub(crate) fn take_pending_foreign() -> Option<ForeignParentInfo> {
    PENDING_FOREIGN.with(|pending| pending.borrow_mut().take())
}

/// Stash the freshly opened foreign window for the embedder to collect.
pub(crate) fn stash_opened_foreign(inner: Rc<WindowsWindowInner>) {
    OPENED_FOREIGN.with(|opened| *opened.borrow_mut() = Some(inner));
}

/// Take the foreign window the last `open_window` stashed, if any.
///
/// Called by the embedder on the host UI thread right after a successful
/// `cx.open_window` that followed [`stash_pending_foreign`]. Returns
/// `None` when the last creation was not a foreign one.
pub fn take_opened_foreign() -> Option<ForeignWindow> {
    OPENED_FOREIGN.with(|opened| opened.borrow_mut().take().map(ForeignWindow::new))
}

/// Embedder handle to a `WS_CHILD` GPUI window inside a host `HWND`.
///
/// All methods run on the host UI thread and act synchronously: the host
/// message loop keeps dispatching `WM_*` input to the child `WndProc`,
/// while frames, resizes, and focus travel through this handle. `Send` is
/// intentionally not implemented (`Rc` internals); move the handle across
/// threads only by handing the raw child `HWND` value to the UI thread.
pub struct ForeignWindow {
    inner: Rc<WindowsWindowInner>,
}

impl ForeignWindow {
    pub(crate) fn new(inner: Rc<WindowsWindowInner>) -> Self {
        Self { inner }
    }

    /// Raw child `HWND` value (`HWND.0 as isize`) for host-side plumbing
    /// (z-order, explicit show/hide, diagnostics).
    #[must_use]
    pub fn child_hwnd(&self) -> isize {
        self.inner.hwnd().0 as isize
    }

    /// Draw one frame synchronously, exactly like a `WM_PAINT` dispatch.
    ///
    /// Returns `false` — and draws nothing — when the child `HWND` is
    /// already gone (the host tore the view down without a matching
    /// destroy call); the embedder counts that as a dropped frame.
    /// Never blocks: GPU contention is handled inside the renderer.
    pub fn request_frame(&self) -> bool {
        let hwnd = self.inner.hwnd();
        if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
            return false;
        }
        self.inner.draw_window(hwnd, false);
        true
    }

    /// Host-driven resize: move the child, resize the swapchain, and fire
    /// the GPUI resize callback.
    ///
    /// `SetWindowPos` delivers `WM_SIZE` synchronously for same-thread
    /// windows, which applies the renderer resize and the GPUI callback;
    /// the explicit converge call afterwards keeps scale-only changes
    /// (which produce no `WM_SIZE`) landing too. The renderer resize
    /// early-outs when the size is unchanged, so the double application
    /// on the size-changed path is a cheap no-op plus one redundant
    /// (idempotent) GPUI bounds update.
    pub fn resize_foreign(&self, width: i32, height: i32, scale: f32) {
        if !scale.is_finite() || scale <= 0.0 {
            log::error!("ForeignWindow::resize_foreign: invalid scale {scale}");
            return;
        }
        let width = width.max(1);
        let height = height.max(1);
        self.inner.state.scale_factor.set(scale);
        let hwnd = self.inner.hwnd();
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .context("foreign child SetWindowPos")
            .log_err();
        }
        let device_size = client_device_size(hwnd);
        self.inner.handle_size_change(device_size, scale, true);
    }

    /// Report host focus changes for GPUI window activation state.
    ///
    /// Child windows never receive `WM_ACTIVATE`, so the host drives this
    /// explicitly. The embedder guards transition-only delivery; repeated
    /// identical values here would re-fire the GPUI callback.
    pub fn set_foreign_active(&self, active: bool) {
        if active {
            // Mirror `handle_activate_msg`: activation resets stale
            // modifier tracking and emits the current modifier state.
            self.inner.state.last_reported_modifiers.set(None);
            self.inner.state.last_reported_capslock.set(None);
            if let Some(mut func) = self.inner.state.callbacks.input.take() {
                func(PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                    modifiers: current_modifiers(),
                    capslock: current_capslock(),
                }));
                self.inner.state.callbacks.input.set(Some(func));
            }
        }
        if let Some(mut func) = self.inner.state.callbacks.active_status_change.take() {
            func(active);
            self.inner
                .state
                .callbacks
                .active_status_change
                .set(Some(func));
        }
    }

    /// Report whether the pointer is currently inside the child view.
    ///
    /// The child already tracks real `WM_MOUSELEAVE` traffic; this entry
    /// exists for hosts that manage hover at the parent level. The
    /// embedder guards transition-only delivery.
    pub fn set_foreign_hovered(&self, hovered: bool) {
        self.inner.state.hovered.set(hovered);
        if let Some(mut func) = self.inner.state.callbacks.hovered_status_change.take() {
            func(hovered);
            self.inner
                .state
                .callbacks
                .hovered_status_change
                .set(Some(func));
        }
    }

    /// Destroy the child window synchronously.
    ///
    /// The host must not use the child `HWND` afterwards. The GPUI-owned
    /// `WindowsWindow` drop later skips the already-destroyed `HWND` via
    /// its `IsWindow` guard.
    pub fn destroy(self) {
        let hwnd = self.inner.hwnd();
        unsafe {
            DestroyWindow(hwnd)
                .context("foreign child DestroyWindow")
                .log_err();
        }
    }
}

/// Current client size of `hwnd` in device pixels, minimum 1 per axis.
fn client_device_size(hwnd: HWND) -> Size<DevicePixels> {
    let mut rect = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut rect)
            .context("foreign child GetClientRect")
            .log_err();
    }
    size(
        DevicePixels((rect.right - rect.left).max(1)),
        DevicePixels((rect.bottom - rect.top).max(1)),
    )
}
