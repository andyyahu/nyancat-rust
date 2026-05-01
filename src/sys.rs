use std::ffi::c_void;
use std::io;

pub const SIGINT: i32 = 2;
pub const SIGPIPE: i32 = 13;
pub const SIGWINCH: i32 = 28;
const POLLIN: i16 = 0x001;

#[cfg(any(target_os = "linux", target_os = "android"))]
const TIOCGWINSZ: usize = 0x5413;

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
const TIOCGWINSZ: usize = 0x4008_7468;

pub type SignalHandler = extern "C" fn(i32);

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: usize, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
    fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
    fn _exit(status: i32) -> !;
}

pub fn install_signal_handler(signum: i32, handler: SignalHandler) {
    unsafe {
        signal(signum, handler as *const () as usize);
    }
}

pub fn stdin_terminal_size() -> Option<(i32, i32)> {
    let mut winsize = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let rc = unsafe { ioctl(0, TIOCGWINSZ, &mut winsize) };
    if rc == 0 && winsize.ws_col > 0 && winsize.ws_row > 0 {
        Some((winsize.ws_col as i32, winsize.ws_row as i32))
    } else {
        None
    }
}

pub fn stdin_ready(timeout_ms: i32) -> bool {
    let mut fds = PollFd {
        fd: 0,
        events: POLLIN,
        revents: 0,
    };

    let rc = unsafe { poll(&mut fds, 1, timeout_ms) };
    rc > 0 && (fds.revents & POLLIN) != 0
}

pub fn read_stdin(buffer: &mut [u8]) -> io::Result<Option<usize>> {
    let bytes_read = unsafe { read(0, buffer.as_mut_ptr().cast(), buffer.len()) };
    if bytes_read > 0 {
        Ok(Some(bytes_read as usize))
    } else if bytes_read == 0 {
        Ok(None)
    } else {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            Ok(None)
        } else {
            Err(err)
        }
    }
}

pub fn write_stdout_raw(data: &[u8]) {
    unsafe {
        let _ = write(1, data.as_ptr().cast(), data.len());
    }
}

pub fn exit(status: i32) -> ! {
    unsafe { _exit(status) }
}
