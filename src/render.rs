use crate::animation::{FRAME_HEIGHT, FRAME_WIDTH, FRAMES};
use crate::cli::Config;
use crate::runtime::take_resize_pending;
use crate::terminal::{TerminalType, terminal_size};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

const EMPTY: &[u8] = b"";
const TEXT_BLOCKS: &[u8] = b"  ";
const CP437_BLOCKS: &[u8] = &[0xdb, 0xdb];
const UTF8_BLOCKS: &[u8] = &[0xe2, 0x96, 0x88, 0xe2, 0x96, 0x88];

pub(crate) struct RenderState {
    terminal_width: i32,
    terminal_height: i32,
    min_row: i32,
    max_row: i32,
    min_col: i32,
    max_col: i32,
    using_automatic_width: bool,
    using_automatic_height: bool,
}

impl RenderState {
    pub(crate) fn new(config: &Config, terminal_width: i32, terminal_height: i32) -> Self {
        let rows = config.crop.rows;
        let cols = config.crop.cols;

        Self {
            terminal_width,
            terminal_height,
            min_row: rows.min_or_default(),
            max_row: rows.max_or_default(),
            min_col: cols.min_or_default(),
            max_col: cols.max_or_default(),
            using_automatic_width: cols.is_automatic_range(),
            using_automatic_height: rows.is_automatic_range(),
        }
    }

    pub(crate) fn finalize_auto_crop(&mut self) {
        if self.using_automatic_width {
            self.recalculate_width();
        }
        if self.using_automatic_height {
            self.recalculate_height();
        }
    }

    fn update_terminal_size(&mut self, width: i32, height: i32) {
        self.terminal_width = width;
        self.terminal_height = height;
        if self.using_automatic_width {
            self.recalculate_width();
        }
        if self.using_automatic_height {
            self.recalculate_height();
        }
    }

    fn recalculate_width(&mut self) {
        self.min_col = (FRAME_WIDTH as i32 - self.terminal_width / 2) / 2;
        self.max_col = (FRAME_WIDTH as i32 + self.terminal_width / 2) / 2;
    }

    fn recalculate_height(&mut self) {
        self.min_row = (FRAME_HEIGHT as i32 - (self.terminal_height - 1)) / 2;
        self.max_row = (FRAME_HEIGHT as i32 + (self.terminal_height - 1)) / 2;
    }
}

pub(crate) struct Palette {
    colors: [&'static [u8]; 256],
    output: Option<&'static [u8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PaletteEntry {
    symbol: u8,
    value: &'static [u8],
}

impl PaletteEntry {
    const fn new(symbol: u8, value: &'static [u8]) -> Self {
        Self { symbol, value }
    }
}

const XTERM_256_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"\x1b[48;5;17m"),
    PaletteEntry::new(b'.', b"\x1b[48;5;231m"),
    PaletteEntry::new(b'\'', b"\x1b[48;5;16m"),
    PaletteEntry::new(b'@', b"\x1b[48;5;230m"),
    PaletteEntry::new(b'$', b"\x1b[48;5;175m"),
    PaletteEntry::new(b'-', b"\x1b[48;5;162m"),
    PaletteEntry::new(b'>', b"\x1b[48;5;196m"),
    PaletteEntry::new(b'&', b"\x1b[48;5;214m"),
    PaletteEntry::new(b'+', b"\x1b[48;5;226m"),
    PaletteEntry::new(b'#', b"\x1b[48;5;118m"),
    PaletteEntry::new(b'=', b"\x1b[48;5;33m"),
    PaletteEntry::new(b';', b"\x1b[48;5;19m"),
    PaletteEntry::new(b'*', b"\x1b[48;5;240m"),
    PaletteEntry::new(b'%', b"\x1b[48;5;175m"),
];

const ANSI_16_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"\x1b[104m"),
    PaletteEntry::new(b'.', b"\x1b[107m"),
    PaletteEntry::new(b'\'', b"\x1b[40m"),
    PaletteEntry::new(b'@', b"\x1b[47m"),
    PaletteEntry::new(b'$', b"\x1b[105m"),
    PaletteEntry::new(b'-', b"\x1b[101m"),
    PaletteEntry::new(b'>', b"\x1b[101m"),
    PaletteEntry::new(b'&', b"\x1b[43m"),
    PaletteEntry::new(b'+', b"\x1b[103m"),
    PaletteEntry::new(b'#', b"\x1b[102m"),
    PaletteEntry::new(b'=', b"\x1b[104m"),
    PaletteEntry::new(b';', b"\x1b[44m"),
    PaletteEntry::new(b'*', b"\x1b[100m"),
    PaletteEntry::new(b'%', b"\x1b[105m"),
];

const LINUX_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"\x1b[25;44m"),
    PaletteEntry::new(b'.', b"\x1b[5;47m"),
    PaletteEntry::new(b'\'', b"\x1b[25;40m"),
    PaletteEntry::new(b'@', b"\x1b[5;47m"),
    PaletteEntry::new(b'$', b"\x1b[5;45m"),
    PaletteEntry::new(b'-', b"\x1b[5;41m"),
    PaletteEntry::new(b'>', b"\x1b[5;41m"),
    PaletteEntry::new(b'&', b"\x1b[25;43m"),
    PaletteEntry::new(b'+', b"\x1b[5;43m"),
    PaletteEntry::new(b'#', b"\x1b[5;42m"),
    PaletteEntry::new(b'=', b"\x1b[25;44m"),
    PaletteEntry::new(b';', b"\x1b[5;44m"),
    PaletteEntry::new(b'*', b"\x1b[5;40m"),
    PaletteEntry::new(b'%', b"\x1b[5;45m"),
];

const FALLBACK_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"\x1b[0;34;44m"),
    PaletteEntry::new(b'.', b"\x1b[1;37;47m"),
    PaletteEntry::new(b'\'', b"\x1b[0;30;40m"),
    PaletteEntry::new(b'@', b"\x1b[1;37;47m"),
    PaletteEntry::new(b'$', b"\x1b[1;35;45m"),
    PaletteEntry::new(b'-', b"\x1b[1;31;41m"),
    PaletteEntry::new(b'>', b"\x1b[1;31;41m"),
    PaletteEntry::new(b'&', b"\x1b[0;33;43m"),
    PaletteEntry::new(b'+', b"\x1b[1;33;43m"),
    PaletteEntry::new(b'#', b"\x1b[1;32;42m"),
    PaletteEntry::new(b'=', b"\x1b[1;34;44m"),
    PaletteEntry::new(b';', b"\x1b[0;34;44m"),
    PaletteEntry::new(b'*', b"\x1b[1;30;40m"),
    PaletteEntry::new(b'%', b"\x1b[1;35;45m"),
];

const VT220_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"::"),
    PaletteEntry::new(b'.', b"@@"),
    PaletteEntry::new(b'\'', b"  "),
    PaletteEntry::new(b'@', b"##"),
    PaletteEntry::new(b'$', b"??"),
    PaletteEntry::new(b'-', b"<>"),
    PaletteEntry::new(b'>', b"##"),
    PaletteEntry::new(b'&', b"=="),
    PaletteEntry::new(b'+', b"--"),
    PaletteEntry::new(b'#', b"++"),
    PaletteEntry::new(b'=', b"~~"),
    PaletteEntry::new(b';', b"$$"),
    PaletteEntry::new(b'*', b";;"),
    PaletteEntry::new(b'%', b"()"),
];

const VT100_ASCII_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"."),
    PaletteEntry::new(b'.', b"@"),
    PaletteEntry::new(b'\'', b" "),
    PaletteEntry::new(b'@', b"#"),
    PaletteEntry::new(b'$', b"?"),
    PaletteEntry::new(b'-', b"O"),
    PaletteEntry::new(b'>', b"#"),
    PaletteEntry::new(b'&', b"="),
    PaletteEntry::new(b'+', b"-"),
    PaletteEntry::new(b'#', b"+"),
    PaletteEntry::new(b'=', b"~"),
    PaletteEntry::new(b';', b"$"),
    PaletteEntry::new(b'*', b";"),
    PaletteEntry::new(b'%', b"o"),
];

const TRUE_COLOR_PALETTE: &[PaletteEntry] = &[
    PaletteEntry::new(b',', b"\x1b[48;2;0;49;105m"),
    PaletteEntry::new(b'.', b"\x1b[48;2;255;255;255m"),
    PaletteEntry::new(b'\'', b"\x1b[48;2;0;0;0m"),
    PaletteEntry::new(b'@', b"\x1b[48;2;255;205;152m"),
    PaletteEntry::new(b'$', b"\x1b[48;2;255;169;255m"),
    PaletteEntry::new(b'-', b"\x1b[48;2;255;76;152m"),
    PaletteEntry::new(b'>', b"\x1b[48;2;255;25;0m"),
    PaletteEntry::new(b'&', b"\x1b[48;2;255;154;0m"),
    PaletteEntry::new(b'+', b"\x1b[48;2;255;240;0m"),
    PaletteEntry::new(b'#', b"\x1b[48;2;40;220;0m"),
    PaletteEntry::new(b'=', b"\x1b[48;2;0;144;255m"),
    PaletteEntry::new(b';', b"\x1b[48;2;104;68;255m"),
    PaletteEntry::new(b'*', b"\x1b[48;2;153;153;153m"),
    PaletteEntry::new(b'%', b"\x1b[48;2;255;163;152m"),
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
        self.colors[entry.symbol as usize] = entry.value;
    }

    fn color(&self, symbol: u8) -> &'static [u8] {
        self.colors[symbol as usize]
    }
}

pub(crate) enum RunOutcome {
    FrameLimitReached { clear_screen: bool },
}

pub(crate) fn run(
    config: Config,
    mut state: RenderState,
    palette: Palette,
) -> io::Result<RunOutcome> {
    let mut stdout = io::stdout().lock();

    if config.set_title {
        stdout.write_all(b"\x1bkNyanyanyanyanyanyanya...\x1b\\")?;
        stdout.write_all(b"\x1b]1;Nyanyanyanyanyanyanya...\x07")?;
        stdout.write_all(b"\x1b]2;Nyanyanyanyanyanyanya...\x07")?;
    }

    if config.clear_screen {
        stdout.write_all(b"\x1b[H\x1b[2J\x1b[?25l")?;
    } else {
        stdout.write_all(b"\x1b[s")?;
    }

    if config.show_intro {
        show_intro(&mut stdout, config.telnet, config.clear_screen)?;
    }

    let start = Instant::now();
    let mut frame_index = 0usize;
    let mut frames_rendered = 0u32;
    let mut buffer = Vec::with_capacity(32 * 1024);

    loop {
        let frame_start = Instant::now();

        if !config.telnet && take_resize_pending() {
            let (width, height) = terminal_size();
            state.update_terminal_size(width, height);
        }

        buffer.clear();
        if config.clear_screen {
            buffer.extend_from_slice(b"\x1b[H");
        } else {
            buffer.extend_from_slice(b"\x1b[u");
        }

        render_frame(
            &mut buffer,
            &config,
            &state,
            &palette,
            frame_index,
            start.elapsed().as_secs(),
        );
        stdout.write_all(&buffer)?;
        stdout.flush()?;

        frames_rendered = frames_rendered.saturating_add(1);
        if config.frame_count != 0 && frames_rendered == config.frame_count {
            return Ok(RunOutcome::FrameLimitReached {
                clear_screen: config.clear_screen,
            });
        }

        frame_index += 1;
        if frame_index == FRAMES.len() {
            frame_index = 0;
        }

        let elapsed = frame_start.elapsed();
        let target_delay = Duration::from_millis(config.delay_ms);
        if let Some(sleep_time) = target_delay.checked_sub(elapsed) {
            thread::sleep(sleep_time);
        }
    }
}

fn render_frame(
    out: &mut Vec<u8>,
    config: &Config,
    state: &RenderState,
    palette: &Palette,
    frame_index: usize,
    elapsed_seconds: u64,
) {
    let mut last = 0u8;
    let frame = FRAMES[frame_index];
    const RAINBOW: &[u8] = b",,>>&&&+++###==;;;,,";

    for y in state.min_row..state.max_row {
        for x in state.min_col..state.max_col {
            let color = if y > 23 && y < 43 && x < 0 {
                // Generate rainbow tail for negative x coordinates (off-screen left)
                let mut mod_x = ((-x + 2) % 16) / 8;
                if (frame_index / 2) % 2 == 1 {
                    mod_x = 1 - mod_x;
                }
                let index = (mod_x + y - 23) as usize;
                RAINBOW.get(index).copied().unwrap_or(b',')
            } else if !(0..FRAME_HEIGHT as i32).contains(&y)
                || !(0..FRAME_WIDTH as i32).contains(&x)
            {
                b','
            } else {
                frame[y as usize].as_bytes()[x as usize]
            };

            match palette.output {
                Some(output) => {
                    let escape = palette.color(color);
                    if color != last && !escape.is_empty() {
                        last = color;
                        out.extend_from_slice(escape);
                    }
                    out.extend_from_slice(output);
                }
                None => {
                    // ASCII mode: palette entries already contain the visual representation.
                    out.extend_from_slice(palette.color(color));
                }
            }
        }
        push_newline(out, config.telnet, 1);
    }

    if config.show_counter {
        let width = (state.terminal_width - 29 - elapsed_seconds.to_string().len() as i32) / 2;
        for _ in 0..width.max(0) {
            out.push(b' ');
        }
        out.extend_from_slice(b"\x1b[1;37m");
        let _ = write!(out, "You have nyaned for {elapsed_seconds} seconds!");
        out.extend_from_slice(b"\x1b[J\x1b[0m");
    }
}

fn show_intro(out: &mut impl Write, telnet: bool, clear_screen: bool) -> io::Result<()> {
    let countdown_clock = 5;

    for k in 0..countdown_clock {
        let mut buffer = Vec::with_capacity(1024);
        push_newline(&mut buffer, telnet, 3);
        buffer
            .extend_from_slice(b"                             \x1b[1mNyancat Telnet Server\x1b[0m");
        push_newline(&mut buffer, telnet, 2);
        buffer.extend_from_slice(
            b"                   written and run by \x1b[1;32mK. Lange\x1b[1;34m @_klange\x1b[0m",
        );
        push_newline(&mut buffer, telnet, 2);
        buffer.extend_from_slice(b"        If things don't look right, try:");
        push_newline(&mut buffer, telnet, 1);
        buffer.extend_from_slice(b"                TERM=fallback telnet ...");
        push_newline(&mut buffer, telnet, 2);
        buffer.extend_from_slice(b"        Or on Windows:");
        push_newline(&mut buffer, telnet, 1);
        buffer.extend_from_slice(b"                telnet -t vtnt ...");
        push_newline(&mut buffer, telnet, 2);
        buffer.extend_from_slice(b"        Problems? Check the website:");
        push_newline(&mut buffer, telnet, 1);
        buffer.extend_from_slice(b"                \x1b[1;34mhttp://nyancat.dakko.us\x1b[0m");
        push_newline(&mut buffer, telnet, 2);
        buffer.extend_from_slice(b"        This is a telnet server, remember your escape keys!");
        push_newline(&mut buffer, telnet, 1);
        buffer.extend_from_slice(b"                \x1b[1;31m^]quit\x1b[0m to exit");
        push_newline(&mut buffer, telnet, 2);
        let _ = writeln!(
            buffer,
            "        Starting in {}...                ",
            countdown_clock - k
        );

        out.write_all(&buffer)?;
        out.flush()?;
        thread::sleep(Duration::from_millis(400));
        if clear_screen {
            out.write_all(b"\x1b[H")?;
        } else {
            out.write_all(b"\x1b[u")?;
        }
    }

    if clear_screen {
        out.write_all(b"\x1b[H\x1b[2J\x1b[?25l")?;
    }

    Ok(())
}

fn push_newline(out: &mut Vec<u8>, telnet: bool, count: usize) {
    for _ in 0..count {
        if telnet {
            out.extend_from_slice(b"\r\0\n");
        } else {
            out.push(b'\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_test_frame(config: &Config, elapsed_seconds: u64) -> Vec<u8> {
        let mut state = RenderState::new(config, 80, 24);
        state.finalize_auto_crop();
        let palette = Palette::new(TerminalType::Vt100Ascii);
        let mut out = Vec::new();

        render_frame(&mut out, config, &state, &palette, 0, elapsed_seconds);

        out
    }

    #[test]
    fn palette_uses_terminal_specific_entries() {
        let palette = Palette::new(TerminalType::Vt100Ascii);

        assert_eq!(palette.color(b','), b".");
        assert_eq!(palette.color(b'@'), b"#");
        assert_eq!(palette.output, None);
    }

    #[test]
    fn block_palettes_share_colors_with_different_outputs() {
        let fallback = Palette::new(TerminalType::Fallback);
        let vtnt = Palette::new(TerminalType::Vtnt);

        assert_eq!(fallback.color(b','), vtnt.color(b','));
        assert_eq!(fallback.output, Some(UTF8_BLOCKS));
        assert_eq!(vtnt.output, Some(CP437_BLOCKS));
    }

    #[test]
    fn unknown_palette_symbols_are_empty() {
        let palette = Palette::new(TerminalType::TrueColor);

        assert_eq!(palette.color(b'?'), EMPTY);
    }

    #[test]
    fn counter_uses_supplied_elapsed_seconds() {
        let config = Config::default();
        let out = render_test_frame(&config, 42);

        assert!(
            out.windows(b"You have nyaned for 42 seconds!".len())
                .any(|window| window == b"You have nyaned for 42 seconds!")
        );
    }

    #[test]
    fn counter_can_be_disabled() {
        let config = Config {
            show_counter: false,
            ..Config::default()
        };
        let out = render_test_frame(&config, 42);

        assert!(
            !out.windows(b"You have nyaned for".len())
                .any(|window| window == b"You have nyaned for")
        );
    }

    #[test]
    fn telnet_mode_uses_telnet_newlines() {
        let config = Config {
            telnet: true,
            show_counter: false,
            ..Config::default()
        };
        let out = render_test_frame(&config, 0);

        assert!(
            out.windows(b"\r\0\n".len())
                .any(|window| window == b"\r\0\n")
        );
        assert!(!out.windows(b"\n\n".len()).any(|window| window == b"\n\n"));
    }
}
