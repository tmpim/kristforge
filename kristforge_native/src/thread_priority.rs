use std::thread::current;
use tracing::debug;

/// Reduce the priority of the current thread.
pub fn deprioritize_thread() {
    unsafe {
        #[cfg(windows)]
        let result = {
            use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};
            let thread = GetCurrentThread();
            SetThreadPriority(thread, -1)
        };

        #[cfg(unix)]
        let result = {
            use libc::{setpriority, PRIO_PROCESS};
            setpriority(PRIO_PROCESS as u32, 0, 5)
        };

        debug!(thread = ?current(), ?result, "Attempted to reduce thread priority");
    }
}
