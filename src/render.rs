use crate::animation::{FRAME_HEIGHT, FRAME_WIDTH, FRAMES};
use crate::cli::Config;
use crate::runtime::{finish, take_resize_pending};
use crate::terminal::terminal_size;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

const EMPTY: &[u8] = b"";
const CP437_BLOCKS: &[u8] = &[0xdb, 0xdb];

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
        Self {
            terminal_width,
            terminal_height,
            min_row: config.min_row,
            max_row: config.max_row,
            min_col: config.min_col,
            max_col: config.max_col,
            using_automatic_width: false,
            using_automatic_height: false,
        }
    }

    pub(crate) fn finalize_auto_crop(&mut self) {
        if self.min_col == self.max_col {
            self.using_automatic_width = true;
            self.recalculate_width();
        }
        if self.min_row == self.max_row {
            self.using_automatic_height = true;
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

impl Palette {
    pub(crate) fn new(ttype: u8) -> Self {
        let mut palette = Self {
            colors: [EMPTY; 256],
            output: Some(b"  "),
        };

        match ttype {
            1 => {
                palette.colors[b',' as usize] = b"\x1b[48;5;17m";
                palette.colors[b'.' as usize] = b"\x1b[48;5;231m";
                palette.colors[b'\'' as usize] = b"\x1b[48;5;16m";
                palette.colors[b'@' as usize] = b"\x1b[48;5;230m";
                palette.colors[b'$' as usize] = b"\x1b[48;5;175m";
                palette.colors[b'-' as usize] = b"\x1b[48;5;162m";
                palette.colors[b'>' as usize] = b"\x1b[48;5;196m";
                palette.colors[b'&' as usize] = b"\x1b[48;5;214m";
                palette.colors[b'+' as usize] = b"\x1b[48;5;226m";
                palette.colors[b'#' as usize] = b"\x1b[48;5;118m";
                palette.colors[b'=' as usize] = b"\x1b[48;5;33m";
                palette.colors[b';' as usize] = b"\x1b[48;5;19m";
                palette.colors[b'*' as usize] = b"\x1b[48;5;240m";
                palette.colors[b'%' as usize] = b"\x1b[48;5;175m";
            }
            2 => {
                palette.colors[b',' as usize] = b"\x1b[104m";
                palette.colors[b'.' as usize] = b"\x1b[107m";
                palette.colors[b'\'' as usize] = b"\x1b[40m";
                palette.colors[b'@' as usize] = b"\x1b[47m";
                palette.colors[b'$' as usize] = b"\x1b[105m";
                palette.colors[b'-' as usize] = b"\x1b[101m";
                palette.colors[b'>' as usize] = b"\x1b[101m";
                palette.colors[b'&' as usize] = b"\x1b[43m";
                palette.colors[b'+' as usize] = b"\x1b[103m";
                palette.colors[b'#' as usize] = b"\x1b[102m";
                palette.colors[b'=' as usize] = b"\x1b[104m";
                palette.colors[b';' as usize] = b"\x1b[44m";
                palette.colors[b'*' as usize] = b"\x1b[100m";
                palette.colors[b'%' as usize] = b"\x1b[105m";
            }
            3 => {
                palette.colors[b',' as usize] = b"\x1b[25;44m";
                palette.colors[b'.' as usize] = b"\x1b[5;47m";
                palette.colors[b'\'' as usize] = b"\x1b[25;40m";
                palette.colors[b'@' as usize] = b"\x1b[5;47m";
                palette.colors[b'$' as usize] = b"\x1b[5;45m";
                palette.colors[b'-' as usize] = b"\x1b[5;41m";
                palette.colors[b'>' as usize] = b"\x1b[5;41m";
                palette.colors[b'&' as usize] = b"\x1b[25;43m";
                palette.colors[b'+' as usize] = b"\x1b[5;43m";
                palette.colors[b'#' as usize] = b"\x1b[5;42m";
                palette.colors[b'=' as usize] = b"\x1b[25;44m";
                palette.colors[b';' as usize] = b"\x1b[5;44m";
                palette.colors[b'*' as usize] = b"\x1b[5;40m";
                palette.colors[b'%' as usize] = b"\x1b[5;45m";
            }
            4 => {
                palette.colors[b',' as usize] = b"\x1b[0;34;44m";
                palette.colors[b'.' as usize] = b"\x1b[1;37;47m";
                palette.colors[b'\'' as usize] = b"\x1b[0;30;40m";
                palette.colors[b'@' as usize] = b"\x1b[1;37;47m";
                palette.colors[b'$' as usize] = b"\x1b[1;35;45m";
                palette.colors[b'-' as usize] = b"\x1b[1;31;41m";
                palette.colors[b'>' as usize] = b"\x1b[1;31;41m";
                palette.colors[b'&' as usize] = b"\x1b[0;33;43m";
                palette.colors[b'+' as usize] = b"\x1b[1;33;43m";
                palette.colors[b'#' as usize] = b"\x1b[1;32;42m";
                palette.colors[b'=' as usize] = b"\x1b[1;34;44m";
                palette.colors[b';' as usize] = b"\x1b[0;34;44m";
                palette.colors[b'*' as usize] = b"\x1b[1;30;40m";
                palette.colors[b'%' as usize] = b"\x1b[1;35;45m";
                palette.output = Some("██".as_bytes());
            }
            5 => {
                palette.colors[b',' as usize] = b"\x1b[0;34;44m";
                palette.colors[b'.' as usize] = b"\x1b[1;37;47m";
                palette.colors[b'\'' as usize] = b"\x1b[0;30;40m";
                palette.colors[b'@' as usize] = b"\x1b[1;37;47m";
                palette.colors[b'$' as usize] = b"\x1b[1;35;45m";
                palette.colors[b'-' as usize] = b"\x1b[1;31;41m";
                palette.colors[b'>' as usize] = b"\x1b[1;31;41m";
                palette.colors[b'&' as usize] = b"\x1b[0;33;43m";
                palette.colors[b'+' as usize] = b"\x1b[1;33;43m";
                palette.colors[b'#' as usize] = b"\x1b[1;32;42m";
                palette.colors[b'=' as usize] = b"\x1b[1;34;44m";
                palette.colors[b';' as usize] = b"\x1b[0;34;44m";
                palette.colors[b'*' as usize] = b"\x1b[1;30;40m";
                palette.colors[b'%' as usize] = b"\x1b[1;35;45m";
                palette.output = Some(CP437_BLOCKS);
            }
            6 => {
                palette.colors[b',' as usize] = b"::";
                palette.colors[b'.' as usize] = b"@@";
                palette.colors[b'\'' as usize] = b"  ";
                palette.colors[b'@' as usize] = b"##";
                palette.colors[b'$' as usize] = b"??";
                palette.colors[b'-' as usize] = b"<>";
                palette.colors[b'>' as usize] = b"##";
                palette.colors[b'&' as usize] = b"==";
                palette.colors[b'+' as usize] = b"--";
                palette.colors[b'#' as usize] = b"++";
                palette.colors[b'=' as usize] = b"~~";
                palette.colors[b';' as usize] = b"$$";
                palette.colors[b'*' as usize] = b";;";
                palette.colors[b'%' as usize] = b"()";
                palette.output = None;
            }
            7 => {
                palette.colors[b',' as usize] = b".";
                palette.colors[b'.' as usize] = b"@";
                palette.colors[b'\'' as usize] = b" ";
                palette.colors[b'@' as usize] = b"#";
                palette.colors[b'$' as usize] = b"?";
                palette.colors[b'-' as usize] = b"O";
                palette.colors[b'>' as usize] = b"#";
                palette.colors[b'&' as usize] = b"=";
                palette.colors[b'+' as usize] = b"-";
                palette.colors[b'#' as usize] = b"+";
                palette.colors[b'=' as usize] = b"~";
                palette.colors[b';' as usize] = b"$";
                palette.colors[b'*' as usize] = b";";
                palette.colors[b'%' as usize] = b"o";
                palette.output = None;
            }
            8 => {
                palette.colors[b',' as usize] = b"\x1b[48;2;0;49;105m";
                palette.colors[b'.' as usize] = b"\x1b[48;2;255;255;255m";
                palette.colors[b'\'' as usize] = b"\x1b[48;2;0;0;0m";
                palette.colors[b'@' as usize] = b"\x1b[48;2;255;205;152m";
                palette.colors[b'$' as usize] = b"\x1b[48;2;255;169;255m";
                palette.colors[b'-' as usize] = b"\x1b[48;2;255;76;152m";
                palette.colors[b'>' as usize] = b"\x1b[48;2;255;25;0m";
                palette.colors[b'&' as usize] = b"\x1b[48;2;255;154;0m";
                palette.colors[b'+' as usize] = b"\x1b[48;2;255;240;0m";
                palette.colors[b'#' as usize] = b"\x1b[48;2;40;220;0m";
                palette.colors[b'=' as usize] = b"\x1b[48;2;0;144;255m";
                palette.colors[b';' as usize] = b"\x1b[48;2;104;68;255m";
                palette.colors[b'*' as usize] = b"\x1b[48;2;153;153;153m";
                palette.colors[b'%' as usize] = b"\x1b[48;2;255;163;152m";
            }
            _ => {}
        }

        palette
    }
}

pub(crate) fn run(config: Config, mut state: RenderState, palette: Palette) -> io::Result<()> {
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

        render_frame(&mut buffer, &config, &state, &palette, frame_index, start);
        stdout.write_all(&buffer)?;
        stdout.flush()?;

        frames_rendered = frames_rendered.saturating_add(1);
        if config.frame_count != 0 && frames_rendered == config.frame_count {
            finish(config.clear_screen);
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
    start: Instant,
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
                    if color != last && !palette.colors[color as usize].is_empty() {
                        last = color;
                        out.extend_from_slice(palette.colors[color as usize]);
                    }
                    out.extend_from_slice(output);
                }
                None => {
                    // ASCII mode: palette.colors already contains the visual representation
                    out.extend_from_slice(palette.colors[color as usize]);
                }
            }
        }
        push_newline(out, config.telnet, 1);
    }

    if config.show_counter {
        let seconds = start.elapsed().as_secs();
        let width = (state.terminal_width - 29 - seconds.to_string().len() as i32) / 2;
        for _ in 0..width.max(0) {
            out.push(b' ');
        }
        out.extend_from_slice(b"\x1b[1;37m");
        let _ = write!(out, "You have nyaned for {seconds} seconds!");
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
