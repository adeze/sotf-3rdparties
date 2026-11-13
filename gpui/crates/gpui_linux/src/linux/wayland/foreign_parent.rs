//! Foreign-parent embedding on Wayland: GPUI editors in host subsurfaces.
//!
//! CLAP-wayland binding of the foreign-parent contract
//! (`docs/foreign-parent-editor-embedding.md` §10, Linux row). The plugin
//! creates its own `wl_surface` + `wl_subsurface` on the *host's* Wayland
//! connection and renders with wgpu/Vulkan; the host owns the loop, the
//! parent surface, and the geometry.
//!
//! # Host-provides contract
//!
//! The host passes [`WaylandForeignParent`](crate::linux::foreign_parent::WaylandForeignParent)
//! at attach:
//!
//! - `display`: a live `wl_display*` for the host connection. Must outlive
//!   the editor. The guest wraps it with `Backend::from_foreign_display`
//!   (guest mode: never closed, never `wl_display_disconnect`ed) and runs
//!   its own event queue; the host must keep dispatching its own queue as
//!   usual — guest events for guest objects are routed to the guest queue
//!   by libwayland, and the guest never reads the wire while the host is
//!   dispatching (pump uses `dispatch_pending` only).
//! - `parent_surface`: a live `wl_surface*` the host keeps mapped for the
//!   editor lifetime. Adopted read-only as the subsurface parent: the guest
//!   never destroys, resizes, or reassigns it, and never touches its event
//!   listener (the adopted id is unmanaged, so guest drop cannot destroy it).
//! - `width`/`height`: logical size in CSS pixels; `scale`: backing scale.
//!   The host owns geometry: the guest renders `max(1, round(logical ×
//!   scale))` device pixels and positions the subsurface at (0, 0) in the
//!   parent. Compositor `PreferredBufferScale` advisories are ignored; the
//!   host scale wins.
//!
//! All entry points run on the host UI thread. Attach performs two local
//! registry roundtrips (fast, Unix socket); afterwards the pump is
//! non-blocking: `dispatch_pending` + flush, one `surface.frame` request
//! per drawn frame, re-armed only after the previous frame callback fires.
//!
//! # Input
//!
//! Unlike VST3/X11 hosts, CLAP-wayland hosts do not forward events, so the
//! guest binds its own seat, pointer, and keyboard on the host connection:
//! pointer motion/buttons/scroll map to the `Mouse*`/`ScrollWheel` inputs
//! (same conversions as the app path, unknown buttons fall back to Left),
//! keys resolve through the compositor keymap with the `"unknown"`
//! fallback, and dead-key composition feeds preedit through the input
//! handler. M6 gaps, documented for follow-up: no touch binding, no
//! key-repeat timers (held keys do not auto-repeat), no text-input
//! protocol binding (preedit works through the input handler only), and no
//! cursor-shape support (the host cursor stays visible).
//!
//! # Wayland protocol notes
//!
//! - The subsurface runs in desync mode: commits apply immediately instead
//!   of waiting for the parent commit, so editor frames never depend on
//!   the host's commit cadence.
//! - Fractional scales use `wp_viewporter` (`set_destination` to the
//!   logical size) when the compositor offers it; otherwise the integer
//!   buffer scale is the rounded host scale.

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::time::Instant;

use anyhow::Context as _;
use calloop::{EventLoop, LoopHandle};
use calloop_wayland_source::WaylandSource;
use collections::HashMap;
use gpui_util::ResultExt as _;
use raw_window_handle as rwh;
use wayland_backend::client::{Backend, ObjectId};
use wayland_backend::protocol::WEnum;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_pointer::AxisSource;
use wayland_client::protocol::{
    wl_callback, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_subcompositor,
    wl_subsurface, wl_surface,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use xkbcommon::xkb::ffi::XKB_KEYMAP_FORMAT_TEXT_V1;
use xkbcommon::xkb::{self, KEYMAP_COMPILE_NO_FLAGS, Keycode};

use super::window::{Callbacks, WaylandSubsurfaceState, WaylandSurfaceState};
use crate::linux::foreign_parent::WaylandForeignParent;
use crate::linux::{
    DOUBLE_CLICK_INTERVAL, LinuxCommon, SCROLL_LINES, capslock_from_xkb, get_xkb_compose_state,
    is_within_click_distance, keystroke_from_xkb, keystroke_underlying_dead_key,
    modifiers_from_xkb,
};
use gpui::{
    AnyWindowHandle, Bounds, Capslock, Decorations, DevicePixels, GpuSpecs, KeyDownEvent,
    KeyUpEvent, Modifiers, ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MouseUpEvent, NavigationDirection, Pixels, PlatformAtlas, PlatformDisplay,
    PlatformInput, PlatformInputHandler, PlatformWindow, Point, PromptButton, PromptLevel,
    RequestFrameOptions, Scene, ScrollDelta, ScrollWheelEvent, Size, TouchPhase, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowVisibility, point, px,
};
use gpui_wgpu::{GpuContext, WgpuRenderer, WgpuSurfaceConfig, wgpu};

/// Evdev-to-xkb keycode offset, shared with the app path.
const MIN_KEYCODE: u32 = 8;

/// Continuous-axis to pixel scaling, shared with the app path.
const AXIS_PIXEL_MODIFIER: f32 = 3.0;

/// Continuous scroll larger than this is still a single wheel tick batch.
fn wl_seat_version(version: u32) -> u32 {
    // We rely on the wl_pointer.frame event (seat v5).
    const WL_SEAT_MIN_VERSION: u32 = 5;
    const WL_SEAT_MAX_VERSION: u32 = 9;

    version.clamp(WL_SEAT_MIN_VERSION, WL_SEAT_MAX_VERSION)
}

/// Linux evdev button codes (`linux/input-event-codes.h`) to GPUI buttons.
///
/// Unknown codes fall back to Left (contract §7) instead of dropping the
/// event, so every host click reaches the guest.
fn linux_button_to_gpui(button: u32) -> MouseButton {
    const BTN_LEFT: u32 = 0x110;
    const BTN_RIGHT: u32 = 0x111;
    const BTN_MIDDLE: u32 = 0x112;
    const BTN_SIDE: u32 = 0x113;
    const BTN_EXTRA: u32 = 0x114;
    const BTN_FORWARD: u32 = 0x115;
    const BTN_BACK: u32 = 0x116;

    match button {
        BTN_LEFT => MouseButton::Left,
        BTN_RIGHT => MouseButton::Right,
        BTN_MIDDLE => MouseButton::Middle,
        BTN_BACK | BTN_SIDE => MouseButton::Navigate(NavigationDirection::Back),
        BTN_FORWARD | BTN_EXTRA => MouseButton::Navigate(NavigationDirection::Forward),
        _ => MouseButton::Left,
    }
}

/// Drawable size in device pixels; never zero on any axis.
fn drawable_size(width: u32, height: u32, scale: f32) -> Size<DevicePixels> {
    Size {
        width: DevicePixels(((f64::from(width) * f64::from(scale)).round() as i32).max(1)),
        height: DevicePixels(((f64::from(height) * f64::from(scale)).round() as i32).max(1)),
    }
}

pub(crate) struct WaylandForeignClientState {
    _loop_handle: LoopHandle<'static, WaylandForeignStatePtr>,
    event_loop: Option<EventLoop<'static, WaylandForeignStatePtr>>,
    qh: QueueHandle<WaylandForeignStatePtr>,
    pub(crate) common: LinuxCommon,
    /// Held so the common power listener's sends never fail; foreign
    /// editors do not consume system power events.
    #[allow(dead_code, reason = "held alive for the power listener channel")]
    power_receiver: calloop::channel::Channel<crate::linux::SystemPowerEvent>,
    gpu_context: GpuContext,
    compositor_gpu: Option<gpui_wgpu::CompositorGpuHint>,

    // Bound host globals. The parent surface is adopted (unmanaged): guest
    // drop never destroys it.
    compositor: wl_compositor::WlCompositor,
    compositor_version: u32,
    subcompositor: wl_subcompositor::WlSubcompositor,
    viewporter: Option<wp_viewporter::WpViewporter>,
    #[allow(dead_code, reason = "held alive for the documented editor lifetime")]
    parent_surface: wl_surface::WlSurface,
    // Held alive: dropping the seat releases it, which would kill the
    // pointer/keyboard objects below.
    #[allow(dead_code, reason = "held alive to keep pointer/keyboard valid")]
    seat: Option<wl_seat::WlSeat>,
    #[allow(dead_code, reason = "held alive; events arrive through Dispatch")]
    pointer: Option<wl_pointer::WlPointer>,
    #[allow(dead_code, reason = "held alive; events arrive through Dispatch")]
    keyboard: Option<wl_keyboard::WlKeyboard>,

    keymap_state: Option<xkb::State>,
    compose_state: Option<xkb::compose::State>,
    pre_edit_text: Option<String>,
    modifiers: Modifiers,
    capslock: Capslock,
    mouse_location: Option<Point<Pixels>>,
    button_pressed: Option<MouseButton>,
    axis_source: AxisSource,
    continuous_scroll_delta: Option<Point<Pixels>>,
    discrete_scroll_delta: Option<Point<f32>>,
    scroll_event_received: bool,
    last_click: Instant,
    last_mouse_button: Option<MouseButton>,
    last_location: Point<Pixels>,
    current_count: usize,

    #[allow(clippy::mutable_key_type)]
    pub(crate) windows: HashMap<ObjectId, WaylandForeignWindowStatePtr>,
    mouse_window: Option<ObjectId>,
    pub(crate) keyboard_window: Option<ObjectId>,

    // The connection must drop last: destroying any Wayland object above
    // sends through this backend, and guest mode forbids dropping it first.
    conn: Connection,
}

#[derive(Clone)]
pub(crate) struct WaylandForeignStatePtr(pub Weak<RefCell<WaylandForeignClientState>>);

impl WaylandForeignStatePtr {
    fn get_client(&self) -> Option<WaylandForeignClient> {
        self.0.upgrade().map(WaylandForeignClient)
    }
}

#[derive(Clone)]
pub(crate) struct WaylandForeignClient(pub Rc<RefCell<WaylandForeignClientState>>);

/// Resolve the window owning `surface_id`, if it is still alive.
fn get_window(
    state: &WaylandForeignClientState,
    surface_id: &ObjectId,
) -> Option<WaylandForeignWindowStatePtr> {
    state.windows.get(surface_id).cloned()
}

impl WaylandForeignClient {
    /// Attach a guest client to the host's Wayland connection.
    ///
    /// Binds compositor, subcompositor, seat, and (when offered) viewporter
    /// on the host display, adopts the host parent surface, and arms the
    /// seat/pointer/keyboard listeners. Two local roundtrips run here so
    /// input is live before attach returns; afterwards the pump never blocks.
    pub(crate) fn new(host: &WaylandForeignParent) -> anyhow::Result<Self> {
        // SAFETY: the host guarantees a live `wl_display*` for the editor
        // lifetime (see the module contract); guest mode never closes it.
        let backend = unsafe { Backend::from_foreign_display(host.display_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, event_queue) = registry_queue_init::<WaylandForeignStatePtr>(&conn)
            .context("Wayland foreign: registry init failed")?;
        let qh = event_queue.handle();

        let mut compositor: Option<(u32, wl_compositor::WlCompositor)> = None;
        let mut subcompositor: Option<wl_subcompositor::WlSubcompositor> = None;
        let mut viewporter: Option<wp_viewporter::WpViewporter> = None;
        let mut seat: Option<wl_seat::WlSeat> = None;
        globals.contents().with_list(|list| {
            for global in list {
                match &global.interface[..] {
                    "wl_compositor" => {
                        if compositor.is_none() {
                            let version = global.version.min(6).max(1);
                            let bound = globals
                                .registry()
                                .bind::<wl_compositor::WlCompositor, _, _>(
                                    global.name,
                                    version,
                                    &qh,
                                    (),
                                );
                            compositor = Some((version, bound));
                        }
                    }
                    "wl_subcompositor" => {
                        if subcompositor.is_none() {
                            subcompositor = Some(
                                globals
                                    .registry()
                                    .bind::<wl_subcompositor::WlSubcompositor, _, _>(
                                        global.name,
                                        1,
                                        &qh,
                                        (),
                                    ),
                            );
                        }
                    }
                    "wp_viewporter" => {
                        if viewporter.is_none() {
                            viewporter = Some(
                                globals
                                    .registry()
                                    .bind::<wp_viewporter::WpViewporter, _, _>(
                                        global.name,
                                        1,
                                        &qh,
                                        (),
                                    ),
                            );
                        }
                    }
                    "wl_seat" => {
                        if seat.is_none() {
                            seat = Some(globals.registry().bind::<wl_seat::WlSeat, _, _>(
                                global.name,
                                wl_seat_version(global.version),
                                &qh,
                                (),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        });

        let Some((compositor_version, compositor)) = compositor else {
            anyhow::bail!("Wayland foreign: host compositor offers no wl_compositor");
        };
        let Some(subcompositor) = subcompositor else {
            anyhow::bail!("Wayland foreign: host compositor offers no wl_subcompositor");
        };

        // Adopt the host parent surface without taking ownership: the id is
        // unmanaged, so guest drop refuses to destroy it, and the host keeps
        // its own listener and queue assignment.
        // SAFETY: the host guarantees a live `wl_surface*` for the editor
        // lifetime; the interface check rejects a wrong-typed pointer.
        let parent_surface = unsafe {
            let id = ObjectId::from_ptr(
                wl_surface::WlSurface::interface(),
                host.parent_surface_ptr().cast(),
            )
            .context("Wayland foreign: parent surface pointer has the wrong interface")?;
            wl_surface::WlSurface::from_id(&conn, id)
                .context("Wayland foreign: cannot adopt the parent surface")?
        };

        let event_loop = EventLoop::<WaylandForeignStatePtr>::try_new()
            .context("Wayland foreign: event loop init failed")?;
        let (common, main_receiver, power_receiver) = LinuxCommon::new(event_loop.get_signal());
        let loop_handle = event_loop.handle();
        loop_handle
            .insert_source(main_receiver, {
                let handle = loop_handle.clone();
                move |event, _, _: &mut WaylandForeignStatePtr| {
                    if let calloop::channel::Event::Msg(runnable) = event {
                        handle.insert_idle(|_| {
                            runnable.run();
                        });
                    }
                }
            })
            .map_err(|err| {
                anyhow::anyhow!("Wayland foreign: foreground task source failed: {err:?}")
            })?;

        let client = Self(Rc::new(RefCell::new(WaylandForeignClientState {
            _loop_handle: loop_handle.clone(),
            event_loop: Some(event_loop),
            qh: qh.clone(),
            common,
            power_receiver,
            gpu_context: Rc::new(RefCell::new(None)),
            compositor_gpu: None,
            compositor,
            compositor_version,
            subcompositor,
            viewporter,
            parent_surface,
            seat,
            pointer: None,
            keyboard: None,
            keymap_state: None,
            compose_state: None,
            pre_edit_text: None,
            modifiers: Modifiers::default(),
            capslock: Capslock::default(),
            mouse_location: None,
            button_pressed: None,
            axis_source: AxisSource::Wheel,
            continuous_scroll_delta: None,
            discrete_scroll_delta: None,
            scroll_event_received: false,
            last_click: Instant::now(),
            last_mouse_button: None,
            last_location: point(px(0.0), px(0.0)),
            current_count: 0,
            windows: HashMap::default(),
            mouse_window: None,
            keyboard_window: None,
            conn: conn.clone(),
        })));

        // Second roundtrip: seat capabilities (and the initial keymap) arrive
        // here so pointer/keyboard objects exist before attach returns.
        let mut event_queue = event_queue;
        let mut state_ptr = client.state_ptr();
        event_queue
            .roundtrip(&mut state_ptr)
            .context("Wayland foreign: seat roundtrip failed")?;

        WaylandSource::new(conn, event_queue)
            .insert(loop_handle)
            .map_err(|err| {
                anyhow::anyhow!("Wayland foreign: WaylandSource insert failed: {err:?}")
            })?;

        Ok(client)
    }

    fn state_ptr(&self) -> WaylandForeignStatePtr {
        WaylandForeignStatePtr(Rc::downgrade(&self.0))
    }

    /// Dispatch pending compositor and executor events without blocking.
    ///
    /// Foreign-parent pump: the host owns the loop, so the embedding calls
    /// this on every frame tick instead of running an event loop. Only
    /// already-queued guest events dispatch; the host keeps reading the wire.
    pub(crate) fn pump_events(&self) {
        let Some(mut event_loop) = self.0.borrow_mut().event_loop.take() else {
            return;
        };
        event_loop
            .dispatch(Some(std::time::Duration::ZERO), &mut self.state_ptr())
            .log_err();
        self.0.borrow_mut().event_loop = Some(event_loop);
    }

    /// Resolve a live window by its GPUI handle (foreign-parent pump/resize).
    pub(crate) fn window_by_handle(
        &self,
        handle: AnyWindowHandle,
    ) -> Option<WaylandForeignWindowStatePtr> {
        let state = self.0.borrow();
        state.windows.values().find_map(|window| {
            (window.handle() == handle && !window.is_destroyed()).then(|| window.clone())
        })
    }

    /// Create a subsurface editor window parented to the host surface.
    ///
    /// Reached from `ForeignParentPlatform::open_window` when the toolkit
    /// stashed a Wayland host record. Geometry comes from the host record,
    /// not from `WindowParams`.
    pub(crate) fn open_foreign_window(
        &self,
        handle: AnyWindowHandle,
        host: &WaylandForeignParent,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let mut state = self.0.borrow_mut();
        let surface = state.compositor.create_surface(&state.qh, ());
        let subsurface =
            state
                .subcompositor
                .get_subsurface(&surface, &state.parent_surface, &state.qh, ());
        subsurface.set_position(0, 0);
        // Desync: editor commits apply immediately instead of waiting for
        // the host's commit cadence.
        subsurface.set_desync();

        let viewport = state
            .viewporter
            .as_ref()
            .map(|viewporter| viewporter.get_viewport(&surface, &state.qh, ()));
        if let Some(viewport) = viewport.as_ref() {
            viewport.set_destination(host.width() as i32, host.height() as i32);
        } else if state.compositor_version >= wl_surface::REQ_SET_BUFFER_SCALE_SINCE {
            surface.set_buffer_scale(host.scale().round().max(1.0) as i32);
        }

        let scale = host.scale();
        let renderer = {
            let raw_window = ForeignRawWindow {
                window: surface.id().as_ptr().cast::<c_void>(),
                display: host.display_ptr(),
            };
            let config = WgpuSurfaceConfig {
                size: drawable_size(host.width(), host.height(), scale),
                transparent: true,
                // Prefer Mailbox to avoid blocking. Falls back to FIFO if Mailbox is unsupported.
                preferred_present_mode: Some(wgpu::PresentMode::Mailbox),
            };
            WgpuRenderer::new(
                state.gpu_context.clone(),
                &raw_window,
                config,
                state.compositor_gpu,
            )?
        };
        surface.commit();

        let window = WaylandForeignWindowStatePtr {
            state: Rc::new(RefCell::new(WaylandForeignWindowState {
                surface: surface.clone(),
                surface_state: WaylandSurfaceState::Subsurface(WaylandSubsurfaceState::new(
                    subsurface,
                )),
                viewport,
                renderer,
                host_display: host.display_ptr(),
                conn: state.conn.clone(),
                qh: state.qh.clone(),
                bounds: Bounds {
                    origin: Point::default(),
                    size: Size {
                        width: px(host.width() as f32),
                        height: px(host.height() as f32),
                    },
                },
                scale,
                input_handler: None,
                active: false,
                hovered: false,
                destroyed: false,
                mouse_position: point(px(0.0), px(0.0)),
                frame_callback: None,
                frame_done_chase: true,
                appearance: WindowAppearance::Light,
                background_appearance: WindowBackgroundAppearance::Opaque,
                handle,
                client: WaylandForeignStatePtr(Rc::downgrade(&self.0)),
            })),
            callbacks: Rc::new(RefCell::new(Callbacks::default())),
        };
        state.windows.insert(surface.id(), window.clone());
        Ok(Box::new(WaylandForeignWindow(window)))
    }
}

/// Small `Send + Sync` handle for wgpu surface creation on a guest surface.
///
/// Mirrors the app path's `RawWindow`; the child surface and host display
/// pointers are valid for the window lifetime.
#[derive(Debug, Clone, Copy)]
struct ForeignRawWindow {
    window: *mut c_void,
    display: *mut c_void,
}

// SAFETY: both pointers are valid for the window lifetime (the host owns
// the display, the window owns the child surface); wgpu only reads them
// during surface creation and recovery.
unsafe impl Send for ForeignRawWindow {}
unsafe impl Sync for ForeignRawWindow {}

impl rwh::HasWindowHandle for ForeignRawWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let window = NonNull::new(self.window).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandWindowHandle::new(window);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl rwh::HasDisplayHandle for ForeignRawWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let display = NonNull::new(self.display).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandDisplayHandle::new(display);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

pub(crate) struct WaylandForeignWindowState {
    surface: wl_surface::WlSurface,
    surface_state: WaylandSurfaceState,
    viewport: Option<wp_viewport::WpViewport>,
    renderer: WgpuRenderer,
    /// Host `wl_display*`, for wgpu surface creation and recovery.
    host_display: *mut c_void,
    conn: Connection,
    qh: QueueHandle<WaylandForeignStatePtr>,
    bounds: Bounds<Pixels>,
    scale: f32,
    input_handler: Option<PlatformInputHandler>,
    active: bool,
    hovered: bool,
    destroyed: bool,
    mouse_position: Point<Pixels>,
    /// Outstanding frame callback, if a frame request is still armed.
    frame_callback: Option<wl_callback::WlCallback>,
    /// False while a frame callback is still outstanding (pacing gate).
    frame_done_chase: bool,
    appearance: WindowAppearance,
    background_appearance: WindowBackgroundAppearance,
    handle: AnyWindowHandle,
    client: WaylandForeignStatePtr,
}

#[derive(Clone)]
pub(crate) struct WaylandForeignWindowStatePtr {
    state: Rc<RefCell<WaylandForeignWindowState>>,
    callbacks: Rc<RefCell<Callbacks>>,
}

impl WaylandForeignWindowStatePtr {
    pub(crate) fn handle(&self) -> AnyWindowHandle {
        self.state.borrow().handle
    }

    fn is_destroyed(&self) -> bool {
        self.state.borrow().destroyed
    }

    // Part of the foreign-parent surface mirroring handle()/is_destroyed();
    // the Wayland host-resize path that consumes it is not wired yet.
    #[allow(dead_code)]
    fn surface_id(&self) -> ObjectId {
        self.state.borrow().surface.id()
    }

    /// Dispatch one normalized input to the GPUI input callback.
    ///
    /// Also serves host-forwarded input: embeddings whose host forwards raw
    /// events (rather than relying on the guest seat) call this directly.
    pub(crate) fn handle_input(&self, input: PlatformInput) {
        let callback = self.callbacks.borrow_mut().input.take();
        if let Some(mut fun) = callback {
            let result = fun(input.clone());
            self.callbacks.borrow_mut().input = Some(fun);
            if !result.propagate {
                return;
            }
        }
        if let PlatformInput::KeyDown(event) = input {
            // only allow shift modifier when inserting text
            if event.keystroke.modifiers.is_subset_of(&Modifiers::shift()) {
                let mut state = self.state.borrow_mut();
                if let Some(mut input_handler) = state.input_handler.take() {
                    if let Some(key_char) = &event.keystroke.key_char {
                        drop(state);
                        input_handler.replace_text_in_range(None, key_char);
                        state = self.state.borrow_mut();
                    }
                    state.input_handler = Some(input_handler);
                }
            }
        }
    }

    pub(crate) fn handle_ime_commit(&self, text: String) {
        let mut state = self.state.borrow_mut();
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            input_handler.replace_text_in_range(None, &text);
            let mut state = self.state.borrow_mut();
            state.input_handler = Some(input_handler);
        }
    }

    pub(crate) fn handle_ime_preedit(&self, text: String) {
        let mut state = self.state.borrow_mut();
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            input_handler.replace_and_mark_text_in_range(None, &text, None);
            let mut state = self.state.borrow_mut();
            state.input_handler = Some(input_handler);
        }
    }

    // Wayland foreign-parent IME forwarding is not wired yet; kept with
    // the rest of the host-input surface (see handle_input).
    #[allow(dead_code)]
    pub(crate) fn handle_ime_unmark(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(mut input_handler) = state.input_handler.take() {
            drop(state);
            input_handler.unmark_text();
            let mut state = self.state.borrow_mut();
            state.input_handler = Some(input_handler);
        }
    }

    pub(crate) fn set_active(&self, active: bool) {
        self.state.borrow_mut().active = active;
        let callback = self.callbacks.borrow_mut().active_status_change.take();
        if let Some(mut fun) = callback {
            fun(active);
            self.callbacks.borrow_mut().active_status_change = Some(fun);
        }
    }

    pub(crate) fn set_hovered(&self, hovered: bool) {
        self.state.borrow_mut().hovered = hovered;
        let callback = self.callbacks.borrow_mut().hover_status_change.take();
        if let Some(mut fun) = callback {
            fun(hovered);
            self.callbacks.borrow_mut().hover_status_change = Some(fun);
        }
    }

    fn set_mouse_position(&self, position: Point<Pixels>) {
        self.state.borrow_mut().mouse_position = position;
    }

    fn set_frame_done(&self) {
        let mut state = self.state.borrow_mut();
        state.frame_callback.take();
        state.frame_done_chase = true;
    }

    /// Host-pumped frame: arm a compositor frame callback and forward one
    /// frame request to GPUI.
    ///
    /// The frame callback is re-armed only after the previous one fired, so
    /// callbacks cannot pile up when the host pumps faster than the
    /// compositor presents; GPUI still gets every forwarded request.
    pub(crate) fn request_foreign_frame(&self) {
        let (surface, qh, surface_id, conn) = {
            let state = self.state.borrow();
            (
                state.surface.clone(),
                state.qh.clone(),
                state.surface.id(),
                state.conn.clone(),
            )
        };
        let mut state = self.state.borrow_mut();
        if state.frame_done_chase {
            state.frame_done_chase = false;
            state.frame_callback = Some(surface.frame(&qh, surface_id));
        }
        drop(state);
        conn.flush().log_err();

        let mut callbacks = self.callbacks.borrow_mut();
        if let Some(fun) = callbacks.request_frame.as_mut() {
            fun(RequestFrameOptions::default());
        }
    }

    /// Host-driven resize of a subsurface editor window.
    ///
    /// `width`/`height` are logical pixels and `scale` is the host backing
    /// scale. Bounds, scale, viewport destination, and drawable update
    /// synchronously, then the GPUI resize callback fires.
    pub(crate) fn resize_foreign(&self, width: u32, height: u32, scale: f32) {
        let (content_size, scale_factor) = {
            let mut state = self.state.borrow_mut();
            state.scale = scale;
            state.bounds.size = Size {
                width: px(width as f32),
                height: px(height as f32),
            };
            if let Some(viewport) = state.viewport.as_ref() {
                viewport.set_destination(width as i32, height as i32);
            }
            state
                .renderer
                .update_drawable_size(drawable_size(width, height, scale));
            (state.bounds.size, state.scale)
        };
        if let Some(ref mut fun) = self.callbacks.borrow_mut().resize {
            fun(content_size, scale_factor);
        }
    }

    fn close(&self) {
        if let Some(fun) = self.callbacks.borrow_mut().close.take() {
            fun();
        }
    }
}

pub(crate) struct WaylandForeignWindow(pub WaylandForeignWindowStatePtr);

impl Drop for WaylandForeignWindow {
    fn drop(&mut self) {
        let mut state = self.0.state.borrow_mut();
        state.destroyed = true;
        let surface_id = state.surface.id();
        let client = state.client.clone();

        state.renderer.destroy();

        // Role object first, then the viewport, then the surface itself —
        // the same order as the app path. The host parent surface is owned
        // by the host and untouched here.
        state.surface_state.destroy();
        if let Some(viewport) = state.viewport.take() {
            viewport.destroy();
        }
        state.surface.destroy();
        drop(state);

        if let Some(client) = client.get_client() {
            client.0.borrow_mut().windows.remove(&surface_id);
            let window = self.0.clone();
            client
                .0
                .borrow()
                .common
                .foreground_executor
                .spawn(async move {
                    window.close();
                })
                .detach();
        }
    }
}

impl rwh::HasWindowHandle for WaylandForeignWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let state = self.0.state.borrow();
        let window = NonNull::new(state.surface.id().as_ptr().cast::<c_void>())
            .ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandWindowHandle::new(window);
        Ok(unsafe { rwh::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl rwh::HasDisplayHandle for WaylandForeignWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let state = self.0.state.borrow();
        let display = NonNull::new(state.host_display).ok_or(rwh::HandleError::Unavailable)?;
        let handle = rwh::WaylandDisplayHandle::new(display);
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(handle.into()) })
    }
}

impl PlatformWindow for WaylandForeignWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.state.borrow().bounds
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.0.state.borrow().bounds)
    }

    fn visibility(&self) -> WindowVisibility {
        if self.0.state.borrow().destroyed {
            WindowVisibility::Hidden
        } else {
            WindowVisibility::Visible
        }
    }

    fn on_visibility_change(&self, callback: Box<dyn FnMut(WindowVisibility)>) {
        self.0.callbacks.borrow_mut().visibility_change = Some(callback);
    }

    fn on_display_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().display_changed = Some(callback);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.0.state.borrow().bounds.size
    }

    /// Guest-initiated resize is a no-op: the host owns geometry, and size
    /// flows host → guest through `resize_foreign` only (contract §5).
    fn resize(&mut self, _size: Size<Pixels>) {}

    fn scale_factor(&self) -> f32 {
        self.0.state.borrow().scale
    }

    fn appearance(&self) -> WindowAppearance {
        self.0.state.borrow().appearance
    }

    /// No output tracking in M6: the host owns the parent surface and its
    /// outputs, and scale flows from the host record instead.
    fn display(&self) -> Option<std::rc::Rc<dyn PlatformDisplay>> {
        None
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.0.state.borrow().mouse_position
    }

    fn modifiers(&self) -> Modifiers {
        self.0
            .state
            .borrow()
            .client
            .get_client()
            .map(|client| client.0.borrow().modifiers)
            .unwrap_or_default()
    }

    fn capslock(&self) -> Capslock {
        self.0
            .state
            .borrow()
            .client
            .get_client()
            .map(|client| client.0.borrow().capslock)
            .unwrap_or_default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.state.borrow_mut().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.state.borrow_mut().input_handler.take()
    }

    /// Prompts are unavailable in the plugin context (contract §11).
    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        None
    }

    /// Activation is compositor-driven through the host parent surface;
    /// there is no toplevel to activate.
    fn activate(&self) {}

    fn is_active(&self) -> bool {
        self.0.state.borrow().active
    }

    fn is_hovered(&self) -> bool {
        self.0.state.borrow().hovered
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.state.borrow().background_appearance
    }

    fn set_title(&mut self, _title: &str) {}

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut state = self.0.state.borrow_mut();
        state.background_appearance = background_appearance;
        state
            .renderer
            .update_transparency(background_appearance != WindowBackgroundAppearance::Opaque);
    }

    fn minimize(&self) {}

    fn zoom(&self) {}

    fn toggle_fullscreen(&self) {}

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.callbacks.borrow_mut().request_frame = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> gpui::DispatchEventResult>) {
        self.0.callbacks.borrow_mut().input = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().active_status_change = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.callbacks.borrow_mut().hover_status_change = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.callbacks.borrow_mut().resize = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().moved = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.callbacks.borrow_mut().should_close = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.callbacks.borrow_mut().close = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.callbacks.borrow_mut().appearance_changed = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        let mut state = self.0.state.borrow_mut();

        if state.renderer.device_lost() {
            let raw_window = ForeignRawWindow {
                window: state.surface.id().as_ptr().cast::<c_void>(),
                display: state.host_display,
            };
            if let Err(err) = state.renderer.recover(&raw_window) {
                log::warn!("GPU recovery failed, will retry on next frame: {err}");
            }
            state.surface.commit();
            state.conn.flush().log_err();
            return;
        }

        let presented = state.renderer.draw(scene);

        // Commit so a desync subsurface presents even when the renderer
        // skipped (mirrors the app path's wlroots workaround).
        if !presented {
            state.surface.commit();
        }
        state.conn.flush().log_err();

        if state.renderer.needs_redraw() {
            state.frame_done_chase = true;
        }
    }

    fn sprite_atlas(&self) -> std::sync::Arc<dyn PlatformAtlas> {
        self.0.state.borrow().renderer.sprite_atlas().clone()
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        self.0
            .state
            .borrow()
            .client
            .get_client()
            .is_some_and(|client| {
                client
                    .0
                    .borrow()
                    .gpu_context
                    .borrow()
                    .as_ref()
                    .is_some_and(|ctx| ctx.supports_dual_source_blending())
            })
    }

    /// Subsurfaces carry no decoration role; zero insets.
    fn window_decorations(&self) -> Decorations {
        Decorations::Server
    }

    /// The subsurface maps on the first committed frame; nothing to map.
    fn map_window(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.0.state.borrow().renderer.gpu_specs().into()
    }

    /// No text-input protocol binding in M6 (preedit flows through the
    /// input handler from local compose instead).
    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}
}

// ── Protocol dispatch ────────────────────────────────────────────────────────

delegate_noop!(WaylandForeignStatePtr: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandForeignStatePtr: ignore wl_subcompositor::WlSubcompositor);
delegate_noop!(WaylandForeignStatePtr: ignore wp_viewporter::WpViewporter);
delegate_noop!(WaylandForeignStatePtr: ignore wp_viewport::WpViewport);
delegate_noop!(WaylandForeignStatePtr: ignore wl_subsurface::WlSubsurface);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandForeignStatePtr {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Globals are bound once at attach; dynamic add/remove is ignored.
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandForeignStatePtr {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The host owns geometry, so compositor advisories (output
        // enter/leave, preferred scale/transform) are intentionally
        // ignored; scale flows from the host record instead.
        match event {
            wl_surface::Event::Enter { .. } => {}
            wl_surface::Event::Leave { .. } => {}
            wl_surface::Event::PreferredBufferScale { .. } => {}
            wl_surface::Event::PreferredBufferTransform { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ObjectId> for WaylandForeignStatePtr {
    fn event(
        state: &mut Self,
        _proxy: &wl_callback::WlCallback,
        event: wl_callback::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_callback is server-destroyed on Done and carries no destructor
        // request; like the app path, route by surface id only.
        let wl_callback::Event::Done { .. } = event else {
            return;
        };
        if let Some(client) = state.get_client() {
            let window = get_window(&client.0.borrow(), data);
            if let Some(window) = window {
                window.set_frame_done();
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandForeignStatePtr {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // Only Capabilities matter here (seat names are irrelevant to input).
        let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        else {
            return;
        };
        let Some(client) = state.get_client() else {
            return;
        };
        let mut state = client.0.borrow_mut();
        if capabilities.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
            state.pointer = Some(seat.get_pointer(qh, ()));
        }
        if capabilities.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
            state.keyboard = Some(seat.get_keyboard(qh, ()));
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandForeignStatePtr {
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per pointer event, mirroring the app path"
    )]
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let Some(client) = state.get_client() else {
            return;
        };
        let mut state = client.0.borrow_mut();
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                // Surface-local coordinates: the child origin is the window origin.
                let position = point(px(surface_x as f32), px(surface_y as f32));
                state.mouse_location = Some(position);
                state.button_pressed = None;
                if let Some(window) = get_window(&state, &surface.id()) {
                    window.set_mouse_position(position);
                    state.mouse_window = Some(surface.id());
                    drop(state);
                    // M6: no cursor-shape support; the host cursor stays visible.
                    window.set_hovered(true);
                }
            }
            wl_pointer::Event::Leave { .. } => {
                if let Some(id) = state.mouse_window.take() {
                    let window = get_window(&state, &id);
                    let input = state.mouse_location.map(|position| {
                        PlatformInput::MouseExited(MouseExitEvent {
                            position,
                            pressed_button: state.button_pressed,
                            modifiers: state.modifiers,
                        })
                    });
                    state.mouse_location = None;
                    state.button_pressed = None;
                    drop(state);
                    if let Some(window) = window {
                        if let Some(input) = input {
                            window.handle_input(input);
                        }
                        window.set_hovered(false);
                    }
                }
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                let Some(id) = state.mouse_window.clone() else {
                    return;
                };
                let position = point(px(surface_x as f32), px(surface_y as f32));
                state.mouse_location = Some(position);
                if let Some(window) = get_window(&state, &id) {
                    window.set_mouse_position(position);
                    let input = PlatformInput::MouseMove(MouseMoveEvent {
                        position,
                        pressed_button: state.button_pressed,
                        modifiers: state.modifiers,
                    });
                    drop(state);
                    window.handle_input(input);
                }
            }
            wl_pointer::Event::Button {
                button,
                state: WEnum::Value(button_state),
                ..
            } => {
                let button = linux_button_to_gpui(button);
                let Some(id) = state.mouse_window.clone() else {
                    return;
                };
                let Some(position) = state.mouse_location else {
                    return;
                };
                match button_state {
                    wl_pointer::ButtonState::Pressed => {
                        let click_elapsed = state.last_click.elapsed();
                        if click_elapsed < DOUBLE_CLICK_INTERVAL
                            && state.last_mouse_button.is_some_and(|prev| prev == button)
                            && is_within_click_distance(state.last_location, position)
                        {
                            state.current_count += 1;
                        } else {
                            state.current_count = 1;
                        }
                        state.last_click = Instant::now();
                        state.last_mouse_button = Some(button);
                        state.last_location = position;
                        state.button_pressed = Some(button);
                        if let Some(window) = get_window(&state, &id) {
                            let input = PlatformInput::MouseDown(MouseDownEvent {
                                button,
                                position,
                                modifiers: state.modifiers,
                                click_count: state.current_count,
                                first_mouse: false,
                            });
                            drop(state);
                            window.handle_input(input);
                        }
                    }
                    wl_pointer::ButtonState::Released => {
                        state.button_pressed = None;
                        if let Some(window) = get_window(&state, &id) {
                            let input = PlatformInput::MouseUp(MouseUpEvent {
                                button,
                                position,
                                modifiers: state.modifiers,
                                click_count: state.current_count,
                            });
                            drop(state);
                            window.handle_input(input);
                        }
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::AxisSource {
                axis_source: WEnum::Value(axis_source),
            } => {
                state.axis_source = axis_source;
            }
            wl_pointer::Event::Axis {
                axis: WEnum::Value(axis),
                value,
                ..
            } => {
                if state.axis_source == AxisSource::Wheel {
                    return;
                }
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                state.scroll_event_received = true;
                let scroll_delta = state
                    .continuous_scroll_delta
                    .get_or_insert(point(px(0.0), px(0.0)));
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += px(value as f32 * AXIS_PIXEL_MODIFIER);
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += px(value as f32 * AXIS_PIXEL_MODIFIER);
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::AxisDiscrete {
                axis: WEnum::Value(axis),
                discrete,
            } => {
                state.scroll_event_received = true;
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                let scroll_delta = state.discrete_scroll_delta.get_or_insert(point(0.0, 0.0));
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += discrete as f32 * SCROLL_LINES;
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += discrete as f32 * SCROLL_LINES;
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::AxisValue120 {
                axis: WEnum::Value(axis),
                value120,
            } => {
                state.scroll_event_received = true;
                let axis = if state.modifiers.shift {
                    wl_pointer::Axis::HorizontalScroll
                } else {
                    axis
                };
                let scroll_delta = state.discrete_scroll_delta.get_or_insert(point(0.0, 0.0));
                let wheel_percent = value120 as f32 / 120.0;
                match axis {
                    wl_pointer::Axis::VerticalScroll => {
                        scroll_delta.y += wheel_percent * SCROLL_LINES;
                    }
                    wl_pointer::Axis::HorizontalScroll => {
                        scroll_delta.x += wheel_percent * SCROLL_LINES;
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::Frame => {
                if !state.scroll_event_received {
                    return;
                }
                state.scroll_event_received = false;
                let continuous = state.continuous_scroll_delta.take();
                let discrete = state.discrete_scroll_delta.take();
                let mouse_window = state.mouse_window.clone();
                let mouse_location = state.mouse_location;
                let modifiers = state.modifiers;
                if let (Some(delta), Some(id), Some(position)) =
                    (continuous, mouse_window.clone(), mouse_location)
                {
                    let window = get_window(&state, &id);
                    drop(state);
                    if let Some(window) = window {
                        window.handle_input(PlatformInput::ScrollWheel(ScrollWheelEvent {
                            position,
                            delta: ScrollDelta::Pixels(delta),
                            modifiers,
                            touch_phase: TouchPhase::Moved,
                        }));
                    }
                } else if let (Some(delta), Some(id), Some(position)) =
                    (discrete, mouse_window, mouse_location)
                {
                    let window = get_window(&state, &id);
                    drop(state);
                    if let Some(window) = window {
                        window.handle_input(PlatformInput::ScrollWheel(ScrollWheelEvent {
                            position,
                            delta: ScrollDelta::Lines(delta),
                            modifiers,
                            touch_phase: TouchPhase::Moved,
                        }));
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandForeignStatePtr {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let Some(client) = state.get_client() else {
            return;
        };
        let mut state = client.0.borrow_mut();
        match event {
            wl_keyboard::Event::Keymap {
                format: WEnum::Value(format),
                fd,
                size,
                ..
            } => {
                if format != wl_keyboard::KeymapFormat::XkbV1 {
                    log::error!("Wayland foreign: keymap format {format:?}, expected XkbV1");
                    return;
                }
                let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
                // SAFETY: the compositor sent a sealed keymap fd of `size` bytes.
                let keymap = unsafe {
                    xkb::Keymap::new_from_fd(
                        &xkb_context,
                        fd,
                        size as usize,
                        XKB_KEYMAP_FORMAT_TEXT_V1,
                        KEYMAP_COMPILE_NO_FLAGS,
                    )
                }
                .log_err()
                .flatten();
                let Some(keymap) = keymap else {
                    log::error!("Wayland foreign: keymap creation failed");
                    return;
                };
                state.keymap_state = Some(xkb::State::new(&keymap));
                state.compose_state = get_xkb_compose_state(&xkb_context);
            }
            wl_keyboard::Event::Enter { surface, .. } => {
                if let Some(window) = get_window(&state, &surface.id()) {
                    state.keyboard_window = Some(surface.id());
                    drop(state);
                    window.set_active(true);
                }
            }
            wl_keyboard::Event::Leave { surface, .. } => {
                let window = get_window(&state, &surface.id());
                state.keyboard_window = None;
                if let Some(compose) = state.compose_state.as_mut() {
                    compose.reset();
                }
                state.pre_edit_text.take();
                drop(state);
                if let Some(window) = window {
                    window.set_active(false);
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                let focused_window = state
                    .keyboard_window
                    .clone()
                    .and_then(|id| get_window(&state, &id));
                let Some(keymap_state) = state.keymap_state.as_mut() else {
                    return;
                };
                keymap_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                state.modifiers = modifiers_from_xkb(keymap_state);
                let keymap_state = state.keymap_state.as_ref();
                state.capslock = keymap_state.map(capslock_from_xkb).unwrap_or_default();
                let input = PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                    modifiers: state.modifiers,
                    capslock: state.capslock,
                });
                drop(state);
                if let Some(window) = focused_window {
                    window.handle_input(input);
                }
            }
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(key_state),
                ..
            } => {
                let window = state
                    .keyboard_window
                    .clone()
                    .and_then(|id| get_window(&state, &id));
                let Some(window) = window else {
                    return;
                };
                let Some(keymap_state) = state.keymap_state.as_ref() else {
                    return;
                };
                let keycode = Keycode::from(key + MIN_KEYCODE);
                let keysym = keymap_state.key_get_one_sym(keycode);
                match key_state {
                    wl_keyboard::KeyState::Pressed if !keysym.is_modifier_key() => {
                        let mut keystroke =
                            keystroke_from_xkb(keymap_state, state.modifiers, keycode);
                        if let Some(mut compose) = state.compose_state.take() {
                            compose.feed(keysym);
                            match compose.status() {
                                xkb::Status::Composing => {
                                    keystroke.key_char = None;
                                    state.pre_edit_text =
                                        compose.utf8().or(keystroke_underlying_dead_key(keysym));
                                    let pre_edit = state.pre_edit_text.clone().unwrap_or_default();
                                    drop(state);
                                    window.handle_ime_preedit(pre_edit);
                                    client.0.borrow_mut().compose_state = Some(compose);
                                    return;
                                }
                                xkb::Status::Composed => {
                                    state.pre_edit_text.take();
                                    keystroke.key_char = compose.utf8();
                                    if let Some(sym) = compose.keysym() {
                                        keystroke.key = xkb::keysym_get_name(sym);
                                    }
                                }
                                xkb::Status::Cancelled => {
                                    let pre_edit = state.pre_edit_text.take();
                                    let underlying = keystroke_underlying_dead_key(keysym);
                                    state.pre_edit_text = underlying.clone();
                                    drop(state);
                                    if let Some(pre_edit) = pre_edit {
                                        window.handle_ime_commit(pre_edit);
                                    }
                                    if let Some(current) = underlying {
                                        window.handle_ime_preedit(current);
                                    }
                                    compose.feed(keysym);
                                    client.0.borrow_mut().compose_state = Some(compose);
                                    return;
                                }
                                xkb::Status::Nothing => {}
                            }
                            state.compose_state = Some(compose);
                        }
                        let input = PlatformInput::KeyDown(KeyDownEvent {
                            keystroke,
                            is_held: false,
                            prefer_character_input: false,
                        });
                        drop(state);
                        // M6: no key-repeat timers; held keys repeat only if
                        // the compositor re-sends Key (most do not).
                        window.handle_input(input);
                    }
                    wl_keyboard::KeyState::Released => {
                        let keystroke = keystroke_from_xkb(keymap_state, state.modifiers, keycode);
                        let input = PlatformInput::KeyUp(KeyUpEvent { keystroke });
                        drop(state);
                        window.handle_input(input);
                    }
                    _ => {}
                }
            }
            // Repeat cadence is stored nowhere in M6 (no repeat timers yet).
            wl_keyboard::Event::RepeatInfo { .. } => {}
            _ => {}
        }
    }
}
