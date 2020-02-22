#[cfg(windows)]
pub fn set_low_priority() {
    use winapi::um::processthreadsapi::*;

    unsafe {
        let thread = GetCurrentThread();
        SetThreadPriority(thread, -1);
    }
}

#[cfg(unix)]
pub fn set_low_priority() {
    use libc::*;

    unsafe {
        assert_eq!(setpriority(PRIO_PROCESS as u32, 0, 5), 0);
    }
}
