//! Shutdown handlers only signal the event loop; no text/allocator work in POSIX handlers.
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Shutdown {
    requested: Arc<AtomicBool>,
    #[cfg(unix)]
    signals: signal_hook::iterator::Handle,
}
impl Shutdown {
    pub fn install(wake: crate::events::Wake) -> io::Result<Self> {
        let requested = Arc::new(AtomicBool::new(false));
        #[cfg(unix)]
        {
            let mut signals = signal_hook::iterator::Signals::new([
                signal_hook::consts::SIGTERM,
                signal_hook::consts::SIGHUP,
            ])?;
            let handle = signals.handle();
            let flag = requested.clone();
            std::thread::Builder::new()
                .name("editio-shutdown".into())
                .stack_size(128 * 1024)
                .spawn(move || {
                    for _ in signals.forever() {
                        arm_exit();
                        flag.store(true, Ordering::Release);
                        wake.notify();
                    }
                })?;
            Ok(Self {
                requested,
                signals: handle,
            })
        }
        #[cfg(windows)]
        {
            let _ = WAKE.set(wake);
            CLOSE.store(false, Ordering::Release);
            DONE.store(false, Ordering::Release);
            if unsafe { SetConsoleCtrlHandler(Some(control), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { requested })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Ok(Self { requested })
        }
    }
    pub fn requested(&self) -> bool {
        #[cfg(windows)]
        if CLOSE.load(Ordering::Relaxed) {
            return true;
        }
        self.requested.load(Ordering::Relaxed)
    }
    pub fn complete(&self) {
        #[cfg(windows)]
        DONE.store(true, Ordering::Release);
    }
}
impl Drop for Shutdown {
    fn drop(&mut self) {
        self.complete();
        #[cfg(unix)]
        self.signals.close();
        #[cfg(windows)]
        unsafe {
            SetConsoleCtrlHandler(Some(control), 0);
        }
    }
}
#[cfg(windows)]
static WAKE: std::sync::OnceLock<crate::events::Wake> = std::sync::OnceLock::new();
#[cfg(windows)]
static CLOSE: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static DONE: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}
#[cfg(windows)]
unsafe extern "system" fn control(event: u32) -> i32 {
    if !matches!(event, 2 | 5 | 6) {
        return 0;
    } // close, logoff, shutdown; Ctrl+C stays Copy
    arm_exit();
    CLOSE.store(true, Ordering::Release);
    if let Some(wake) = WAKE.get() {
        wake.notify();
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
    while !DONE.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    1
}

// Armed only for terminal loss, fatal errors, or accepted termination. Do not
// cancel before process exit: terminal restoration/destructors can also stall.
static EXIT_ARMED: AtomicBool = AtomicBool::new(false);
pub fn arm_exit() {
    arm_after(std::time::Duration::from_secs(5));
}
fn arm_after(deadline: std::time::Duration) {
    if EXIT_ARMED.swap(true, Ordering::AcqRel) {
        return;
    }
    if std::thread::Builder::new()
        .name("editio-exit-deadline".into())
        .stack_size(128 * 1024)
        .spawn(move || {
            std::thread::sleep(deadline);
            force_exit();
        })
        .is_err()
    {
        // If no watchdog can be created, termination must not depend on a stuck
        // main thread. Previously committed drafts remain on disk.
        force_exit();
    }
}
fn force_exit() -> ! {
    // Bypass stdio flushing and destructors: either can be the original stall.
    #[cfg(unix)]
    unsafe {
        libc::_exit(1)
    }
    #[cfg(windows)]
    unsafe {
        TerminateProcess(GetCurrentProcess(), 1);
        std::process::abort();
    }
    #[cfg(not(any(unix, windows)))]
    std::process::abort();
}
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn TerminateProcess(process: *mut std::ffi::c_void, code: u32) -> i32;
}
pub fn guard_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        arm_exit();
        previous(info);
    }));
}

/// Declare after the terminal session so early returns arm the deadline before
/// terminal restoration. It does nothing until this standalone invocation ends.
pub struct OnExit;
impl Drop for OnExit {
    fn drop(&mut self) {
        arm_exit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "subprocess helper; executed by deadline test"]
    fn stalled_shutdown_helper() {
        arm_exit();
        loop {
            std::thread::park();
        }
    }
    #[test]
    fn shutdown_deadline_terminates_a_stalled_process() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "shutdown::tests::stalled_shutdown_helper",
                "--ignored",
            ])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(1));
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("shutdown deadline did not terminate blocked child");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
