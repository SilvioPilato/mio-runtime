//! mio-runtime — a single-threaded, callback-based I/O event loop built on `mio`.

mod event_loop;
mod handler;
mod registry;
mod timerwheel;
mod types;

pub use event_loop::{EventLoop, Waker};
pub use handler::EventHandler;
pub use registry::Registry;
#[doc(hidden)]
pub use timerwheel::TimerWheel;
pub use types::{ReadyState, TimerId, Token};
