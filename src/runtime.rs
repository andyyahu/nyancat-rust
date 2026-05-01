use crate::sys;
use std::io::{self, Write};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

static CLEAR_SCREEN_ON_EXIT: AtomicBool = AtomicBool::new(true);
static RESIZE_PENDING: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_clear_screen_on_exit(clear_screen: bool) {
    CLEAR_SCREEN_ON_EXIT.store(clear_screen, Ordering::Relaxed);
}

pub(crate) fn clear_screen_on_exit() -> bool {
    CLEAR_SCREEN_ON_EXIT.load(Ordering::Relaxed)
}

pub(crate) fn take_resize_pending() -> bool {
    RESIZE_PENDING.swap(false, Ordering::Relaxed)
}

pub(crate) fn install_signal_handlers() {
    sys::install_signal_handler(sys::SIGINT, handle_exit_signal);
    sys::install_signal_handler(sys::SIGPIPE, handle_exit_signal);
    sys::install_signal_handler(sys::SIGWINCH, handle_resize_signal);
}

extern "C" fn handle_exit_signal(_: i32) {
    let sequence = if CLEAR_SCREEN_ON_EXIT.load(Ordering::Relaxed) {
        b"\x1b[?25h\x1b[0m\x1b[H\x1b[2J" as &[u8]
    } else {
        b"\x1b[0m\n" as &[u8]
    };

    sys::write_stdout_raw(sequence);
    sys::exit(0);
}

extern "C" fn handle_resize_signal(_: i32) {
    RESIZE_PENDING.store(true, Ordering::Relaxed);
}

pub(crate) fn finish(clear_screen: bool) -> ! {
    let mut stdout = io::stdout().lock();
    if clear_screen {
        let _ = stdout.write_all(b"\x1b[?25h\x1b[0m\x1b[H\x1b[2J");
    } else {
        let _ = stdout.write_all(b"\x1b[0m\n");
    }
    let _ = stdout.flush();
    process::exit(0);
}
