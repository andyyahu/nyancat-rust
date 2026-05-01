use crate::sys;

pub(crate) fn terminal_size() -> (i32, i32) {
    let mut winsize = sys::Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let rc = unsafe { sys::ioctl(0, sys::TIOCGWINSZ, &mut winsize) };
    if rc == 0 && winsize.ws_col > 0 && winsize.ws_row > 0 {
        (winsize.ws_col as i32, winsize.ws_row as i32)
    } else {
        (80, 24)
    }
}

pub(crate) fn detect_terminal_type(term: Option<&str>, terminal_width: i32) -> u8 {
    let Some(term) = term else {
        return 2;
    };
    let term = term.to_ascii_lowercase();

    if term.contains("xterm") || term.contains("toaru") {
        1
    } else if term.contains("linux") {
        3
    } else if term.contains("vtnt") || term.contains("cygwin") {
        5
    } else if term.contains("vt220") {
        6
    } else if term.contains("fallback") {
        4
    } else if term.contains("rxvt-256color") {
        1
    } else if term.contains("rxvt") {
        3
    } else if term.contains("vt100") && terminal_width == 40 {
        7
    } else if term.starts_with("st") {
        1
    } else if term.starts_with("truecolor") {
        8
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_expected_terminal_types() {
        assert_eq!(detect_terminal_type(Some("xterm-256color"), 80), 1);
        assert_eq!(detect_terminal_type(Some("linux"), 80), 3);
        assert_eq!(detect_terminal_type(Some("vt220"), 80), 6);
        assert_eq!(detect_terminal_type(Some("vt100"), 40), 7);
        assert_eq!(detect_terminal_type(None, 80), 2);
    }
}
