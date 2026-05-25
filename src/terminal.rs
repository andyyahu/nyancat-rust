use crate::sys;
use std::num::NonZeroU16;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TerminalSize {
    width: NonZeroU16,
    height: NonZeroU16,
}

impl TerminalSize {
    pub(crate) const fn new(width: u16, height: u16) -> Self {
        let (Some(width), Some(height)) = (NonZeroU16::new(width), NonZeroU16::new(height)) else {
            panic!("terminal dimensions must be non-zero");
        };

        Self { width, height }
    }

    pub(crate) fn try_new(width: i32, height: i32) -> Option<Self> {
        let width = u16::try_from(width).ok().and_then(NonZeroU16::new)?;
        let height = u16::try_from(height).ok().and_then(NonZeroU16::new)?;

        Some(Self { width, height })
    }

    pub(crate) fn with_width(self, width: u16) -> Self {
        Self {
            width: NonZeroU16::new(width).expect("terminal width must be non-zero"),
            ..self
        }
    }

    pub(crate) const fn width(self) -> i32 {
        self.width.get() as i32
    }

    pub(crate) const fn height(self) -> i32 {
        self.height.get() as i32
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

pub(crate) fn terminal_size() -> TerminalSize {
    sys::stdin_terminal_size()
        .and_then(|(width, height)| TerminalSize::try_new(width, height))
        .unwrap_or_default()
}

pub(crate) fn detect_terminal_type(term: Option<&str>, size: TerminalSize) -> TerminalType {
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
    } else if term.contains("vt100") && size.width() == 40 {
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
            detect_terminal_type(Some("xterm-256color"), TerminalSize::new(80, 24)),
            TerminalType::Xterm256
        );
        assert_eq!(
            detect_terminal_type(Some("linux"), TerminalSize::new(80, 24)),
            TerminalType::Linux
        );
        assert_eq!(
            detect_terminal_type(Some("vt220"), TerminalSize::new(80, 24)),
            TerminalType::Vt220
        );
        assert_eq!(
            detect_terminal_type(Some("vt100"), TerminalSize::new(40, 24)),
            TerminalType::Vt100Ascii
        );
        assert_eq!(
            detect_terminal_type(None, TerminalSize::new(80, 24)),
            TerminalType::Ansi16
        );
    }

    #[test]
    fn terminal_size_defaults_to_standard_dimensions() {
        assert_eq!(TerminalSize::default(), TerminalSize::new(80, 24));
        assert_eq!(
            TerminalSize::new(80, 24).with_width(40),
            TerminalSize::new(40, 24)
        );
        assert_eq!(TerminalSize::new(80, 24).width(), 80);
        assert_eq!(TerminalSize::new(80, 24).height(), 24);
        assert_eq!(
            TerminalSize::try_new(80, 24),
            Some(TerminalSize::new(80, 24))
        );
        assert_eq!(TerminalSize::try_new(0, 24), None);
        assert_eq!(TerminalSize::try_new(80, -1), None);
    }
}
