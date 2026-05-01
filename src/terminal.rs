use crate::sys;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalType {
    Xterm256,
    Ansi16,
    Linux,
    Fallback,
    Vtnt,
    Vt220,
    Vt100Ascii,
    TrueColor,
}

pub(crate) fn terminal_size() -> (i32, i32) {
    sys::stdin_terminal_size().unwrap_or((80, 24))
}

pub(crate) fn detect_terminal_type(term: Option<&str>, terminal_width: i32) -> TerminalType {
    let Some(term) = term else {
        return TerminalType::Ansi16;
    };
    let term = term.to_ascii_lowercase();

    if term.contains("xterm") || term.contains("toaru") {
        TerminalType::Xterm256
    } else if term.contains("linux") {
        TerminalType::Linux
    } else if term.contains("vtnt") || term.contains("cygwin") {
        TerminalType::Vtnt
    } else if term.contains("vt220") {
        TerminalType::Vt220
    } else if term.contains("fallback") {
        TerminalType::Fallback
    } else if term.contains("rxvt-256color") {
        TerminalType::Xterm256
    } else if term.contains("rxvt") {
        TerminalType::Linux
    } else if term.contains("vt100") && terminal_width == 40 {
        TerminalType::Vt100Ascii
    } else if term.starts_with("st") {
        TerminalType::Xterm256
    } else if term.starts_with("truecolor") {
        TerminalType::TrueColor
    } else {
        TerminalType::Ansi16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_expected_terminal_types() {
        assert_eq!(
            detect_terminal_type(Some("xterm-256color"), 80),
            TerminalType::Xterm256
        );
        assert_eq!(detect_terminal_type(Some("linux"), 80), TerminalType::Linux);
        assert_eq!(detect_terminal_type(Some("vt220"), 80), TerminalType::Vt220);
        assert_eq!(
            detect_terminal_type(Some("vt100"), 40),
            TerminalType::Vt100Ascii
        );
        assert_eq!(detect_terminal_type(None, 80), TerminalType::Ansi16);
    }
}
