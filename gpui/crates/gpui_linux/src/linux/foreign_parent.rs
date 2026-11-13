//! Public foreign-parent embedding API for Linux.
//!
//! [`ForeignParentPlatform`] runs GPUI editors inside host-owned X11 windows
//! (VST3, CLAP-x11) and Wayland surfaces (CLAP-wayland), following
//! `docs/foreign-parent-editor-embedding.md`. The host owns the loop, the
//! parent, and the geometry; the guest creates exactly one undecorated
//! child (X11) or desync subsurface (Wayland) per editor and renders into
//! it with wgpu.
//!
//! Embedding flow (see `gpui-linux-foreign` for the full lifecycle and C
//! ABI): stash the host record with [`stash_pending_x11_foreign_parent`]
//! or [`stash_pending_wayland_foreign_parent`], build the platform with
//! [`ForeignParentPlatform::new_x11`] or
//! [`ForeignParentPlatform::new_wayland`], run the GPUI `Application` on
//! it (its `run` invokes the launch callback synchronously and returns),
//! open one window (which takes the pending record one-shot), then drive
//! [`ForeignParentPlatform::pump_events`] and
//! [`ForeignParentPlatform::request_frame`] from the host frame timer.
//!
//! All entry points run on the host UI thread.

use std::cell::RefCell;
#[cfg(feature = "wayland")]
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::anyhow;

#[cfg(feature = "wayland")]
use super::wayland::foreign_parent::WaylandForeignClient;
#[cfg(feature = "x11")]
use super::x11::{X11Connection, X11ForeignChildParams};
use super::{LinuxCommon, LinuxKeyboardLayout};
use gpui::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, ForegroundExecutor,
    Keymap, Menu, MenuItem, Platform, PlatformDisplay, PlatformInput, PlatformKeyboardLayout,
    PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, Task, ThermalState,
    WindowAppearance, WindowParams,
};

#[cfg(feature = "x11")]
thread_local! {
    static PENDING_X11_FOREIGN: RefCell<Option<X11ForeignParent>> =
        const { RefCell::new(None) };
}

#[cfg(feature = "wayland")]
thread_local! {
    static PENDING_WAYLAND_FOREIGN: RefCell<Option<WaylandForeignParent>> =
        const { RefCell::new(None) };
}

/// Host record for an X11 foreign-parent editor (VST3, CLAP-x11).
///
/// The host provides its own window XID plus the initial logical size and
/// backing scale. The guest creates one undecorated `INPUT_OUTPUT` child at
/// parent origin (0, 0), sized `max(1, round(logical × scale))` device
/// pixels, selects the usual input masks plus XInput2 on the child, and
/// registers it in the window map so `ConfigureNotify`, focus, and input
/// events flow through the shared handlers. Mirrors nih-plug's
/// `ParentWindowHandle::X11Window(u32)`.
#[cfg(feature = "x11")]
#[derive(Debug, Clone, Copy)]
pub struct X11ForeignParent {
    parent: u32,
    width: u32,
    height: u32,
    scale: f32,
}

#[cfg(feature = "x11")]
impl X11ForeignParent {
    /// Build a validated host record; `None` for a zero parent, zero size,
    /// or non-finite/non-positive scale.
    #[must_use]
    pub fn new(parent: u32, width: u32, height: u32, scale: f32) -> Option<Self> {
        if parent == 0 || width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        Some(Self {
            parent,
            width,
            height,
            scale,
        })
    }

    /// Host-owned parent window (XID).
    #[must_use]
    pub fn parent(&self) -> u32 {
        self.parent
    }

    /// Initial logical size in CSS pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Host backing scale (device pixels per logical pixel).
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub(crate) fn child_params(&self) -> X11ForeignChildParams {
        X11ForeignChildParams {
            parent: self.parent,
            width: self.width,
            height: self.height,
            scale: self.scale,
        }
    }
}

/// Stash the X11 host record for the next `open_window` call.
///
/// One-shot: `open_window` takes it, so a later app window never inherits
/// a stale parent. Call on the host UI thread, immediately before opening
/// the editor window.
#[cfg(feature = "x11")]
pub fn stash_pending_x11_foreign_parent(info: X11ForeignParent) {
    PENDING_X11_FOREIGN.with(|pending| *pending.borrow_mut() = Some(info));
}

/// Take the stashed X11 host record, if the embedding stashed one.
#[cfg(feature = "x11")]
pub(crate) fn take_pending_x11_foreign_parent() -> Option<X11ForeignChildParams> {
    PENDING_X11_FOREIGN.with(|pending| pending.borrow_mut().take().map(|info| info.child_params()))
}

/// Drop a stashed X11 host record without opening a window.
///
/// Call when attach fails after stashing, so a later window on this thread
/// never inherits the stale parent.
#[cfg(feature = "x11")]
pub fn clear_pending_x11_foreign_parent() {
    PENDING_X11_FOREIGN.with(|pending| *pending.borrow_mut() = None);
}

/// Host record for a Wayland foreign-parent editor (CLAP-wayland).
///
/// The exact host-provides contract (see `wayland::foreign_parent` for the
/// full protocol notes):
///
/// - `display`: a live `wl_display*` for the host connection, valid for
///   the editor lifetime. Wrapped with `Backend::from_foreign_display`
///   (guest mode: never closed); the host keeps dispatching its own queue.
/// - `parent_surface`: a live `wl_surface*` the host keeps mapped for the
///   editor lifetime. Adopted read-only as the subsurface parent; never
///   destroyed, resized, or re-listened by the guest.
/// - `width`/`height`: logical size in CSS pixels; `scale`: backing scale.
///   The host owns geometry; the guest renders `max(1, round(logical ×
///   scale))` device pixels at parent origin (0, 0).
///
/// CLAP hosts pass the complementary pieces of `clap_window` for
/// `CLAP_WINDOW_API_WAYLAND`; hosts that only track the parent surface
/// derive the display with `wl_proxy_get_display`.
#[cfg(feature = "wayland")]
#[derive(Clone, Copy)]
pub struct WaylandForeignParent {
    display: *mut c_void,
    parent_surface: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
}

// SAFETY: the raw pointers name host-owned objects that outlive the editor
// by contract; every use runs on the host UI thread only.
#[cfg(feature = "wayland")]
unsafe impl Send for WaylandForeignParent {}
#[cfg(feature = "wayland")]
unsafe impl Sync for WaylandForeignParent {}

#[cfg(feature = "wayland")]
impl std::fmt::Debug for WaylandForeignParent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaylandForeignParent")
            .field("display", &self.display)
            .field("parent_surface", &self.parent_surface)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scale", &self.scale)
            .finish()
    }
}

#[cfg(feature = "wayland")]
impl WaylandForeignParent {
    /// Build a validated host record; `None` for null pointers, zero size,
    /// or non-finite/non-positive scale.
    ///
    /// # Safety
    ///
    /// `display` must be null or a live `wl_display*`, and
    /// `parent_surface` must be null or a live `wl_surface*` on that
    /// display; both must outlive the editor built from this record.
    #[must_use]
    pub unsafe fn new(
        display: *mut c_void,
        parent_surface: *mut c_void,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Option<Self> {
        if display.is_null()
            || parent_surface.is_null()
            || width == 0
            || height == 0
            || !scale.is_finite()
            || scale <= 0.0
        {
            return None;
        }
        Some(Self {
            display,
            parent_surface,
            width,
            height,
            scale,
        })
    }

    /// Host `wl_display*`.
    #[must_use]
    pub fn display_ptr(&self) -> *mut c_void {
        self.display
    }

    /// Host parent `wl_surface*`.
    #[must_use]
    pub fn parent_surface_ptr(&self) -> *mut c_void {
        self.parent_surface
    }

    /// Initial logical width in CSS pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Initial logical height in CSS pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Host backing scale (device pixels per logical pixel).
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

/// Stash the Wayland host record for the next `open_window` call.
///
/// One-shot: `open_window` takes it, so a later window never inherits a
/// stale parent. Call on the host UI thread, immediately before opening
/// the editor window.
#[cfg(feature = "wayland")]
pub fn stash_pending_wayland_foreign_parent(info: WaylandForeignParent) {
    PENDING_WAYLAND_FOREIGN.with(|pending| *pending.borrow_mut() = Some(info));
}

/// Take the stashed Wayland host record, if the embedding stashed one.
#[cfg(feature = "wayland")]
pub(crate) fn take_pending_wayland_foreign_parent() -> Option<WaylandForeignParent> {
    PENDING_WAYLAND_FOREIGN.with(|pending| pending.borrow_mut().take())
}

/// Drop a stashed Wayland host record without opening a window.
///
/// Call when attach fails after stashing, so a later window on this thread
/// never inherits the stale parent.
#[cfg(feature = "wayland")]
pub fn clear_pending_wayland_foreign_parent() {
    PENDING_WAYLAND_FOREIGN.with(|pending| *pending.borrow_mut() = None);
}

pub(crate) enum ForeignParentClient {
    #[cfg(feature = "x11")]
    X11(X11Connection),
    #[cfg(feature = "wayland")]
    Wayland(WaylandForeignClient),
}

/// GPUI platform for host-owned parents (VST3/CLAP editors on Linux).
///
/// One platform per editor: it owns the native connection facet (X11) or
/// the guest queue on the host display (Wayland), opens exactly the
/// foreign windows the toolkit requests, and pumps on host ticks. `run`
/// invokes the launch callback synchronously and returns — the host loop
/// is never started, nested, or stopped.
pub struct ForeignParentPlatform {
    client: ForeignParentClient,
}

impl ForeignParentPlatform {
    /// Build an X11 foreign-parent platform on the current display.
    ///
    /// Fails when no X display is reachable; the caller reports a null
    /// context and the host keeps running.
    #[cfg(feature = "x11")]
    pub fn new_x11() -> anyhow::Result<Self> {
        Ok(Self {
            client: ForeignParentClient::X11(X11Connection::new_foreign()?),
        })
    }

    /// Build a Wayland foreign-parent platform on the host display.
    ///
    /// Binds the guest queue, compositor, subcompositor, seat, and
    /// viewporter on the host connection; fails cleanly when any required
    /// global is missing.
    #[cfg(feature = "wayland")]
    pub fn new_wayland(host: &WaylandForeignParent) -> anyhow::Result<Self> {
        Ok(Self {
            client: ForeignParentClient::Wayland(WaylandForeignClient::new(host)?),
        })
    }

    fn with_common<R>(&self, f: impl FnOnce(&mut LinuxCommon) -> R) -> R {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.with_common(f),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => f(&mut client.0.borrow_mut().common),
        }
    }

    /// Dispatch pending native and executor events without blocking.
    ///
    /// Call from the host frame tick before
    /// [`ForeignParentPlatform::request_frame`].
    pub fn pump_events(&self) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.dispatch_pending(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => client.pump_events(),
        }
    }

    /// True when `window` names a live foreign window of this platform.
    pub fn has_window(&self, window: AnyWindowHandle) -> bool {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.window_by_handle(window).is_some(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => client.window_by_handle(window).is_some(),
        }
    }

    /// Forward one frame request for `window` (host-pumped frames).
    ///
    /// Unknown or destroyed handles are ignored.
    pub fn request_frame(&self, window: AnyWindowHandle) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.refresh(gpui::RequestFrameOptions::default());
                }
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.request_foreign_frame();
                }
            }
        }
    }

    /// Host-driven resize: logical `width`/`height` plus backing `scale`.
    ///
    /// Bounds, scale, and drawable update synchronously, then the GPUI
    /// resize callback fires. Unknown handles are ignored.
    pub fn resize_foreign(&self, window: AnyWindowHandle, width: u32, height: u32, scale: f32) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.resize_foreign(width, height, scale);
                }
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.resize_foreign(width, height, scale);
                }
            }
        }
    }

    /// Report host focus changes. Unknown handles are ignored.
    pub fn set_foreign_active(&self, window: AnyWindowHandle, active: bool) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.set_active(active);
                }
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.set_active(active);
                }
            }
        }
    }

    /// Report host pointer enter/leave. Unknown handles are ignored.
    pub fn set_foreign_hovered(&self, window: AnyWindowHandle, hovered: bool) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.set_hovered(hovered);
                }
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.set_hovered(hovered);
                }
            }
        }
    }

    /// Deliver one host-forwarded input to `window`.
    ///
    /// Used by hosts that forward raw events; the Wayland guest seat feeds
    /// the same path directly. Unknown handles are ignored.
    pub fn input_foreign(&self, window: AnyWindowHandle, input: PlatformInput) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.handle_input(input);
                }
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                if let Some(window) = client.window_by_handle(window) {
                    window.handle_input(input);
                }
            }
        }
    }
}

impl Platform for ForeignParentPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.with_common(|common| common.background_executor.clone())
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.with_common(|common| common.foreground_executor.clone())
    }

    fn text_system(&self) -> std::sync::Arc<dyn PlatformTextSystem> {
        self.with_common(|common| common.text_system.clone())
    }

    /// Invoke the launch callback synchronously and return: the host owns
    /// the loop, so it is never started, nested, or stopped here.
    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        on_finish_launching();
    }

    fn quit(&self) {
        self.with_common(|common| common.signal.stop());
    }

    /// Restarting the host process from a plugin would be catastrophic, so
    /// this is a loud no-op by design.
    fn restart(&self, _binary_path: Option<PathBuf>, _arguments: Vec<std::ffi::OsString>) {
        log::error!("ForeignParentPlatform::restart is unavailable in the plugin context");
    }

    fn activate(&self, _ignoring_other_apps: bool) {}

    fn hide(&self) {}

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.displays(),
            // M6: no output tracking on the Wayland guest queue; scale
            // flows from the host record instead.
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => Vec::new(),
        }
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.primary_display(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => None,
        }
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.active_window(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                let state = client.0.borrow();
                state
                    .keyboard_window
                    .as_ref()
                    .and_then(|id| state.windows.get(id))
                    .map(|window| window.handle())
            }
        }
    }

    /// Open the foreign window for the stashed host record.
    ///
    /// Fails when no record was stashed: this platform only opens
    /// host-parented windows, never app windows.
    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => {
                let Some(foreign) = take_pending_x11_foreign_parent() else {
                    anyhow::bail!(
                        "ForeignParentPlatform opens host-parented windows only; stash an X11ForeignParent first"
                    )
                };
                client.open_foreign_child_window(handle, options, foreign)
            }
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(client) => {
                let Some(host) = take_pending_wayland_foreign_parent() else {
                    anyhow::bail!(
                        "ForeignParentPlatform opens host-parented windows only; stash a WaylandForeignParent first"
                    )
                };
                client.open_foreign_window(handle, &host)
            }
        }
    }

    fn window_appearance(&self) -> WindowAppearance {
        self.with_common(|common| common.appearance)
    }

    fn open_url(&self, url: &str) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.open_uri(url),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {
                super::open_uri_internal(
                    self.background_executor(),
                    url,
                    None,
                    gpui::GraphicalEnvironment::detect(),
                );
            }
        }
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.with_common(|common| common.callbacks.open_urls = Some(callback));
    }

    fn register_url_scheme(&self, _: &str) -> Task<anyhow::Result<()>> {
        Task::ready(Err(anyhow!(
            "register_url_scheme is unavailable in the plugin context"
        )))
    }

    fn prompt_for_paths(
        &self,
        _options: gpui::PathPromptOptions,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Option<Vec<PathBuf>>>> {
        let (done_tx, done_rx) = futures::channel::oneshot::channel();
        let _ = done_tx.send(Ok(None));
        done_rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<Option<PathBuf>>> {
        let (done_tx, done_rx) = futures::channel::oneshot::channel();
        let _ = done_tx.send(Ok(None));
        done_rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, path: &Path) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.reveal_path(path.to_owned()),
            // Portal pickers need a toplevel parent ident; unavailable here.
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {
                log::info!(
                    "ForeignParentPlatform::reveal_path is unavailable in the plugin context"
                );
            }
        }
    }

    fn open_with_system(&self, path: &Path) {
        let path = path.to_owned();
        self.background_executor()
            .spawn(async move {
                #[allow(
                    clippy::disallowed_methods,
                    reason = "running on a background thread, so blocking is fine"
                )]
                gpui_util::new_std_command("xdg-open")
                    .arg(path)
                    .status()
                    .map(|_| ())
                    .unwrap_or(())
            })
            .detach();
    }

    fn on_quit(&self, callback: Box<dyn FnMut() -> bool>) {
        self.with_common(|common| common.callbacks.quit = Some(callback));
    }

    fn on_displays_changed(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| common.callbacks.displays_changed = Some(callback));
    }

    fn on_system_sleep(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| {
            common.callbacks.system_sleep = Some(callback);
            common.start_power_listener();
        });
    }

    fn on_system_wake(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| {
            common.callbacks.system_wake = Some(callback);
            common.start_power_listener();
        });
    }

    fn prevent_idle_sleep(&self, reason: &str) -> Task<anyhow::Result<gpui::ActivityGuard>> {
        let executor = self.background_executor();
        let (guard_tx, guard_rx) = futures::channel::oneshot::channel();
        executor
            .spawn({
                let executor = executor.clone();
                let reason = reason.to_owned();
                async move {
                    guard_tx
                        .send(super::inhibit_idle_sleep(reason, executor).await)
                        .ok();
                }
            })
            .detach();
        executor
            .clone()
            .spawn(async move { super::await_idle_sleep_prevention(guard_rx, &executor).await })
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| common.callbacks.reopen = Some(callback));
    }

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.with_common(|common| common.callbacks.app_menu_action = Some(callback));
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| common.callbacks.will_open_app_menu = Some(callback));
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.with_common(|common| common.callbacks.validate_app_menu_command = Some(callback));
    }

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }

    fn on_thermal_state_change(&self, _callback: Box<dyn FnMut()>) {}

    fn compositor_name(&self) -> &'static str {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.compositor_name(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => "Wayland",
        }
    }

    fn app_path(&self) -> anyhow::Result<PathBuf> {
        Ok(std::env::current_exe()?)
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> anyhow::Result<PathBuf> {
        Err(anyhow!(
            "ForeignParentPlatform::path_for_auxiliary_executable is unavailable in the plugin context"
        ))
    }

    fn set_cursor_style(&self, style: CursorStyle) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.set_cursor_style(style),
            // M6: no cursor-shape support on the guest queue.
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {}
        }
    }

    fn hide_cursor_until_mouse_moves(&self) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.hide_cursor_until_mouse_moves(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {}
        }
    }

    fn is_cursor_visible(&self) -> bool {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.is_cursor_visible(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => true,
        }
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        self.with_common(|common| common.auto_hide_scrollbars)
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.read_from_clipboard(),
            // M6: no data-device binding on the guest queue.
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => None,
        }
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.write_to_clipboard(item),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {}
        }
    }

    fn read_from_primary(&self) -> Option<ClipboardItem> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.read_from_primary(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => None,
        }
    }

    fn write_to_primary(&self, item: ClipboardItem) {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.write_to_primary(item),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => {}
        }
    }

    fn write_credentials(
        &self,
        _url: &str,
        _username: &str,
        _password: &[u8],
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Err(anyhow!(
            "credentials are unavailable in the plugin context"
        )))
    }

    fn read_credentials(&self, _url: &str) -> Task<anyhow::Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Ok(None))
    }

    fn delete_credentials(&self, _url: &str) -> Task<anyhow::Result<()>> {
        Task::ready(Err(anyhow!(
            "credentials are unavailable in the plugin context"
        )))
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        match &self.client {
            #[cfg(feature = "x11")]
            ForeignParentClient::X11(client) => client.keyboard_layout(),
            #[cfg(feature = "wayland")]
            ForeignParentClient::Wayland(_) => Box::new(LinuxKeyboardLayout::new("unknown".into())),
        }
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(gpui::DummyKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.with_common(|common| common.callbacks.keyboard_layout_change = Some(callback));
    }
}
