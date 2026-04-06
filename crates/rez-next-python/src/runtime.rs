//! Shared Tokio runtime for all Python bindings.
//!
//! A single `tokio::runtime::Runtime` is created once (via `OnceLock`) and
//! reused across every Python-exposed async call. This avoids the overhead of
//! spinning up a new thread-pool on every call into the extension module.

use std::sync::OnceLock;

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Return a reference to the process-wide Tokio runtime.
///
/// The first call initialises the runtime; subsequent calls return the same
/// instance.  The returned reference has `'static` lifetime so callers can
/// use `block_on` without any lifetime trouble.
pub(crate) fn get_runtime() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        tokio::runtime::Runtime::new()
            .expect("Failed to create shared Tokio runtime for rez-next-python")
    })
}
