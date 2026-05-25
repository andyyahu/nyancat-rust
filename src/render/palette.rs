use crate::animation::FrameSymbol;
use crate::terminal::TerminalType;

const EMPTY: &[u8] = b"";
const TEXT_BLOCKS: &[u8] = b"  ";
const CP437_BLOCKS: &[u8] = &[0xdb, 0xdb];
const UTF8_BLOCKS: &[u8] = &[0xe2, 0x96, 0x88, 0xe2, 0x96, 0x88];

pub(crate) struct Palette {
    colors: [&'static [u8]; 256],
    pub(super) output: Option<&'static [u8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PaletteEntry {
    symbol: FrameSymbol,
    value: &'static [u8],
}

impl PaletteEntry {
    const fn new(symbol: FrameSymbol, value: &'static [u8]) -> Self {
        Self { symbol, value }
    }
}

const XTERM_256_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"\x1b[48;5;17m"),
    PaletteEntry::new(FrameSymbol::STAR, b"\x1b[48;5;231m"),
    PaletteEntry::new(FrameSymbol::BLACK, b"\x1b[48;5;16m"),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"\x1b[48;5;230m"),
    PaletteEntry::new(FrameSymbol::BODY, b"\x1b[48;5;175m"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"\x1b[48;5;162m"),
    PaletteEntry::new(FrameSymbol::RED, b"\x1b[48;5;196m"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"\x1b[48;5;214m"),
    PaletteEntry::new(FrameSymbol::YELLOW, b"\x1b[48;5;226m"),
    PaletteEntry::new(FrameSymbol::GREEN, b"\x1b[48;5;118m"),
    PaletteEntry::new(FrameSymbol::BLUE, b"\x1b[48;5;33m"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"\x1b[48;5;19m"),
    PaletteEntry::new(FrameSymbol::FACE, b"\x1b[48;5;240m"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"\x1b[48;5;175m"),
];

const ANSI_16_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"\x1b[104m"),
    PaletteEntry::new(FrameSymbol::STAR, b"\x1b[107m"),
    PaletteEntry::new(FrameSymbol::BLACK, b"\x1b[40m"),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"\x1b[47m"),
    PaletteEntry::new(FrameSymbol::BODY, b"\x1b[105m"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"\x1b[101m"),
    PaletteEntry::new(FrameSymbol::RED, b"\x1b[101m"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"\x1b[43m"),
    PaletteEntry::new(FrameSymbol::YELLOW, b"\x1b[103m"),
    PaletteEntry::new(FrameSymbol::GREEN, b"\x1b[102m"),
    PaletteEntry::new(FrameSymbol::BLUE, b"\x1b[104m"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"\x1b[44m"),
    PaletteEntry::new(FrameSymbol::FACE, b"\x1b[100m"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"\x1b[105m"),
];

const LINUX_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"\x1b[25;44m"),
    PaletteEntry::new(FrameSymbol::STAR, b"\x1b[5;47m"),
    PaletteEntry::new(FrameSymbol::BLACK, b"\x1b[25;40m"),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"\x1b[5;47m"),
    PaletteEntry::new(FrameSymbol::BODY, b"\x1b[5;45m"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"\x1b[5;41m"),
    PaletteEntry::new(FrameSymbol::RED, b"\x1b[5;41m"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"\x1b[25;43m"),
    PaletteEntry::new(FrameSymbol::YELLOW, b"\x1b[5;43m"),
    PaletteEntry::new(FrameSymbol::GREEN, b"\x1b[5;42m"),
    PaletteEntry::new(FrameSymbol::BLUE, b"\x1b[25;44m"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"\x1b[5;44m"),
    PaletteEntry::new(FrameSymbol::FACE, b"\x1b[5;40m"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"\x1b[5;45m"),
];

const FALLBACK_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"\x1b[0;34;44m"),
    PaletteEntry::new(FrameSymbol::STAR, b"\x1b[1;37;47m"),
    PaletteEntry::new(FrameSymbol::BLACK, b"\x1b[0;30;40m"),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"\x1b[1;37;47m"),
    PaletteEntry::new(FrameSymbol::BODY, b"\x1b[1;35;45m"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"\x1b[1;31;41m"),
    PaletteEntry::new(FrameSymbol::RED, b"\x1b[1;31;41m"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"\x1b[0;33;43m"),
    PaletteEntry::new(FrameSymbol::YELLOW, b"\x1b[1;33;43m"),
    PaletteEntry::new(FrameSymbol::GREEN, b"\x1b[1;32;42m"),
    PaletteEntry::new(FrameSymbol::BLUE, b"\x1b[1;34;44m"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"\x1b[0;34;44m"),
    PaletteEntry::new(FrameSymbol::FACE, b"\x1b[1;30;40m"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"\x1b[1;35;45m"),
];

const VT220_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"::"),
    PaletteEntry::new(FrameSymbol::STAR, b"@@"),
    PaletteEntry::new(FrameSymbol::BLACK, b"  "),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"##"),
    PaletteEntry::new(FrameSymbol::BODY, b"??"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"<>"),
    PaletteEntry::new(FrameSymbol::RED, b"##"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"=="),
    PaletteEntry::new(FrameSymbol::YELLOW, b"--"),
    PaletteEntry::new(FrameSymbol::GREEN, b"++"),
    PaletteEntry::new(FrameSymbol::BLUE, b"~~"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"$$"),
    PaletteEntry::new(FrameSymbol::FACE, b";;"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"()"),
];

const VT100_ASCII_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"."),
    PaletteEntry::new(FrameSymbol::STAR, b"@"),
    PaletteEntry::new(FrameSymbol::BLACK, b" "),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"#"),
    PaletteEntry::new(FrameSymbol::BODY, b"?"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"O"),
    PaletteEntry::new(FrameSymbol::RED, b"#"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"="),
    PaletteEntry::new(FrameSymbol::YELLOW, b"-"),
    PaletteEntry::new(FrameSymbol::GREEN, b"+"),
    PaletteEntry::new(FrameSymbol::BLUE, b"~"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"$"),
    PaletteEntry::new(FrameSymbol::FACE, b";"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"o"),
];

const TRUE_COLOR_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(FrameSymbol::BACKGROUND, b"\x1b[48;2;0;49;105m"),
    PaletteEntry::new(FrameSymbol::STAR, b"\x1b[48;2;255;255;255m"),
    PaletteEntry::new(FrameSymbol::BLACK, b"\x1b[48;2;0;0;0m"),
    PaletteEntry::new(FrameSymbol::BODY_EDGE, b"\x1b[48;2;255;205;152m"),
    PaletteEntry::new(FrameSymbol::BODY, b"\x1b[48;2;255;169;255m"),
    PaletteEntry::new(FrameSymbol::BODY_MARK, b"\x1b[48;2;255;76;152m"),
    PaletteEntry::new(FrameSymbol::RED, b"\x1b[48;2;255;25;0m"),
    PaletteEntry::new(FrameSymbol::ORANGE, b"\x1b[48;2;255;154;0m"),
    PaletteEntry::new(FrameSymbol::YELLOW, b"\x1b[48;2;255;240;0m"),
    PaletteEntry::new(FrameSymbol::GREEN, b"\x1b[48;2;40;220;0m"),
    PaletteEntry::new(FrameSymbol::BLUE, b"\x1b[48;2;0;144;255m"),
    PaletteEntry::new(FrameSymbol::INDIGO, b"\x1b[48;2;104;68;255m"),
    PaletteEntry::new(FrameSymbol::FACE, b"\x1b[48;2;153;153;153m"),
    PaletteEntry::new(FrameSymbol::BLUSH, b"\x1b[48;2;255;163;152m"),
];

impl Palette {
    pub(crate) fn new(terminal_type: TerminalType) -> Self {
        let (entries, output) = match terminal_type {
            TerminalType::Xterm256 => (XTERM_256_PALETTE, Some(TEXT_BLOCKS)),
            TerminalType::Ansi16 => (ANSI_16_PALETTE, Some(TEXT_BLOCKS)),
            TerminalType::Linux => (LINUX_PALETTE, Some(TEXT_BLOCKS)),
            TerminalType::Fallback => (FALLBACK_PALETTE, Some(UTF8_BLOCKS)),
            TerminalType::Vtnt => (FALLBACK_PALETTE, Some(CP437_BLOCKS)),
            TerminalType::Vt220 => (VT220_PALETTE, None),
            TerminalType::Vt100Ascii => (VT100_ASCII_PALETTE, None),
            TerminalType::TrueColor => (TRUE_COLOR_PALETTE, Some(TEXT_BLOCKS)),
        };

        Self::from_entries(entries, output)
    }

    fn from_entries(entries: &'static [PaletteEntry], output: Option<&'static [u8]>) -> Self {
        let mut palette = Self {
            colors: [EMPTY; 256],
            output,
        };

        for &entry in entries {
            palette.set(entry);
        }

        palette
    }

    fn set(&mut self, entry: PaletteEntry) {
        self.colors[entry.symbol.as_byte() as usize] = entry.value;
    }

    #[inline]
    pub(super) fn color(&self, symbol: FrameSymbol) -> &'static [u8] {
        self.colors[symbol.as_byte() as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_uses_terminal_specific_entries() {
        let palette = Palette::new(TerminalType::Vt100Ascii);

        assert_eq!(palette.color(FrameSymbol::BACKGROUND), b".");
        assert_eq!(palette.color(FrameSymbol::BODY_EDGE), b"#");
        assert_eq!(palette.output, None);
    }

    #[test]
    fn block_palettes_share_colors_with_different_outputs() {
        let fallback = Palette::new(TerminalType::Fallback);
        let vtnt = Palette::new(TerminalType::Vtnt);

        assert_eq!(
            fallback.color(FrameSymbol::BACKGROUND),
            vtnt.color(FrameSymbol::BACKGROUND)
        );
        assert_eq!(fallback.output, Some(UTF8_BLOCKS));
        assert_eq!(vtnt.output, Some(CP437_BLOCKS));
    }

    #[test]
    fn unknown_palette_symbols_are_empty() {
        let palette = Palette::new(TerminalType::TrueColor);

        assert_eq!(palette.colors[b'?' as usize], EMPTY);
    }
}
