use std::ffi::c_void;

pub const SIGINT: i32 = 2;
pub const SIGPIPE: i32 = 13;
pub const SIGWINCH: i32 = 28;
pub const POLLIN: i16 = 0x001;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub const TIOCGWINSZ: usize = 0x5413;

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub const TIOCGWINSZ: usize = 0x4008_7468;

#[repr(C)]
pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[repr(C)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

unsafe extern "C" {
    pub fn ioctl(fd: i32, request: usize, ...) -> i32;
    pub fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    pub fn signal(signum: i32, handler: usize) -> usize;
    pub fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    pub fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
    pub fn _exit(status: i32) -> !;
}

pub fn write_stdout_raw(data: &[u8]) {
    unsafe {
        let _ = write(1, data.as_ptr().cast(), data.len());
    }
}

pub fn exit(status: i32) -> ! {
    unsafe { _exit(status) }
}
