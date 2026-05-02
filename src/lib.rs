//! mio-runtime — a single-threaded, callback-based I/O event loop built on `mio`.

mod timewheel;
mod types;
pub use timewheel::TimeWheel;
pub use types::{ReadyState, TimerId, Token};
