// Platform-independent sizing logic; also compiled on the host under
// `cfg(test)` so its unit tests run without a wasm toolchain.
#[cfg(any(target_family = "wasm", test))]
mod canvas_size;
#[cfg(target_family = "wasm")]
mod dispatcher;
#[cfg(target_family = "wasm")]
mod display;
#[cfg(target_family = "wasm")]
mod events;
#[cfg(target_family = "wasm")]
mod http_client;
#[cfg(target_family = "wasm")]
mod keyboard;
#[cfg(target_family = "wasm")]
mod logging;
#[cfg(target_family = "wasm")]
mod platform;
#[cfg(target_family = "wasm")]
mod window;

#[cfg(target_family = "wasm")]
pub use dispatcher::WebDispatcher;
#[cfg(target_family = "wasm")]
pub use display::WebDisplay;
#[cfg(target_family = "wasm")]
pub use http_client::FetchHttpClient;
#[cfg(target_family = "wasm")]
pub use keyboard::WebKeyboardLayout;
#[cfg(target_family = "wasm")]
pub use logging::init_logging;
#[cfg(target_family = "wasm")]
pub use platform::WebPlatform;
#[cfg(target_family = "wasm")]
pub use window::WebWindow;
