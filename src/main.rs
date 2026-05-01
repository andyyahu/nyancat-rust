mod animation;

use animation::{FRAMES, FRAME_HEIGHT, FRAME_WIDTH};
use std::env;
use std::io::{self, Write};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const EMPTY: &[u8] = b"";
const CP437_BLOCKS: &[u8] = &[0xdb, 0xdb];

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SE: u8 = 240;
const NOP: u8 = 241;
const SB: u8 = 250;

const ECHO: u8 = 1;
const SGA: u8 = 3;
const TTYPE: u8 = 24;
const NAWS: u8 = 31;
const LINEMODE: u8 = 34;
const NEW_ENVIRON: u8 = 39;
const SEND: u8 = 1;

static CLEAR_SCREEN_ON_EXIT: AtomicBool = AtomicBool::new(true);
static RESIZE_PENDING: AtomicBool = AtomicBool::new(false);

pub mod sys {
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
}

struct TimeoutReader {
    buffer: [u8; 1024],
    head: usize,
    tail: usize,
}

impl TimeoutReader {
    fn new() -> Self {
        Self {
            buffer: [0; 1024],
            head: 0,
            tail: 0,
        }
    }

    fn read_byte(&mut self, deadline: std::time::Instant) -> std::io::Result<Option<u8>> {
        if self.head < self.tail {
            let byte = self.buffer[self.head];
            self.head += 1;
            return Ok(Some(byte));
        }

        let now = std::time::Instant::now();
        if now >= deadline {
            return Ok(None);
        }

        let timeout = deadline.saturating_duration_since(now);
        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;

        let mut fds = sys::PollFd {
            fd: 0,
            events: sys::POLLIN,
            revents: 0,
        };

        let rc = unsafe { sys::poll(&mut fds, 1, timeout_ms) };
        if rc > 0 && (fds.revents & sys::POLLIN) != 0 {
            let bytes_read =
                unsafe { sys::read(0, self.buffer.as_mut_ptr().cast(), self.buffer.len()) };
            if bytes_read > 0 {
                self.head = 1;
                self.tail = bytes_read as usize;
                return Ok(Some(self.buffer[0]));
            } else if bytes_read == 0 {
                return Ok(None);
            } else {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    return Ok(None);
                }
                return Err(err);
            }
        }

        Ok(None)
    }
}

struct Config {
    telnet: bool,
    show_counter: bool,
    frame_count: u32,
    clear_screen: bool,
    set_title: bool,
    show_intro: bool,
    skip_intro: bool,
    delay_ms: u64,
    min_row: i32,
    max_row: i32,
    min_col: i32,
    max_col: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            telnet: false,
            show_counter: true,
            frame_count: 0,
            clear_screen: true,
            set_title: true,
            show_intro: false,
            skip_intro: false,
            delay_ms: 90,
            min_row: -1,
            max_row: -1,
            min_col: -1,
            max_col: -1,
        }
    }
}

struct RenderState {
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
    fn new(config: &Config, terminal_width: i32, terminal_height: i32) -> Self {
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

    fn finalize_auto_crop(&mut self) {
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

struct Palette {
    colors: [&'static [u8]; 256],
    output: Option<&'static [u8]>,
}

impl Palette {
    fn new(ttype: u8) -> Self {
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

struct TelnetState {
    options: [u8; 256],
    willack: [u8; 256],
    do_set: [u8; 256],
    will_set: [u8; 256],
}

impl TelnetState {
    fn new() -> Self {
        let mut state = Self {
            options: [0; 256],
            willack: [0; 256],
            do_set: [0; 256],
            will_set: [0; 256],
        };

        state.options[ECHO as usize] = WONT;
        state.options[SGA as usize] = WILL;
        state.options[NEW_ENVIRON as usize] = WONT;
        state.willack[ECHO as usize] = DO;
        state.willack[SGA as usize] = DO;
        state.willack[NAWS as usize] = DO;
        state.willack[TTYPE as usize] = DO;
        state.willack[LINEMODE as usize] = DONT;
        state.willack[NEW_ENVIRON as usize] = DO;

        state
    }

    fn send_command(&mut self, out: &mut impl Write, cmd: u8, opt: u8) -> io::Result<()> {
        match cmd {
            DO | DONT => {
                let current = self.do_set[opt as usize];
                if (cmd == DO && current != DO) || (cmd == DONT && current != DONT) {
                    self.do_set[opt as usize] = cmd;
                    out.write_all(&[IAC, cmd, opt])?;
                }
            }
            WILL | WONT => {
                let current = self.will_set[opt as usize];
                if (cmd == WILL && current != WILL) || (cmd == WONT && current != WONT) {
                    self.will_set[opt as usize] = cmd;
                    out.write_all(&[IAC, cmd, opt])?;
                }
            }
            _ => out.write_all(&[IAC, cmd])?,
        }
        Ok(())
    }
}

fn main() {
    let mut config = Config::default();
    let args: Vec<String> = env::args().collect();
    parse_args(&args, &mut config);

    if config.telnet && !config.skip_intro {
        config.show_intro = true;
    }

    CLEAR_SCREEN_ON_EXIT.store(config.clear_screen, Ordering::Relaxed);
    install_signal_handlers();

    let (term, mut terminal_width, terminal_height) = if config.telnet {
        let mut stdout = io::stdout().lock();
        let info = match negotiate_telnet(&mut stdout) {
            Ok(info) => info,
            Err(_) => finish(config.clear_screen),
        };
        (
            info.term,
            info.width.unwrap_or(80),
            info.height.unwrap_or(24),
        )
    } else {
        let (width, height) = terminal_size();
        (env::var("TERM").ok(), width, height)
    };

    let ttype = detect_terminal_type(term.as_deref(), terminal_width);
    if ttype == 7 {
        terminal_width = 40;
    }

    let palette = Palette::new(ttype);
    let mut state = RenderState::new(&config, terminal_width, terminal_height);
    state.finalize_auto_crop();

    if let Err(error) = run(config, state, palette) {
        if error.kind() == io::ErrorKind::BrokenPipe {
            finish(CLEAR_SCREEN_ON_EXIT.load(Ordering::Relaxed));
        }
        let _ = writeln!(io::stderr(), "nyancat: {error}");
        finish(CLEAR_SCREEN_ON_EXIT.load(Ordering::Relaxed));
    }
}

fn run(config: Config, mut state: RenderState, palette: Palette) -> io::Result<()> {
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

        if !config.telnet && RESIZE_PENDING.swap(false, Ordering::Relaxed) {
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

struct TelnetInfo {
    term: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
}

fn negotiate_telnet(out: &mut impl Write) -> io::Result<TelnetInfo> {
    let mut state = TelnetState::new();

    for option in 0..=255u8 {
        let cmd_opt = state.options[option as usize];
        if cmd_opt != 0 {
            state.send_command(out, cmd_opt, option)?;
        }
        let cmd_willack = state.willack[option as usize];
        if cmd_willack != 0 {
            state.send_command(out, cmd_willack, option)?;
        }
    }
    out.flush()?;

    let mut input = TimeoutReader::new();
    let mut deadline = Instant::now() + Duration::from_secs(1);
    let mut got_ttype = false;
    let mut got_naws = false;
    let mut sb_mode = false;
    let mut sb = Vec::with_capacity(1024);
    let mut info = TelnetInfo {
        term: None,
        width: None,
        height: None,
    };

    while !got_ttype || !got_naws {
        let Some(byte) = input.read_byte(deadline)? else {
            break;
        };

        if byte == IAC {
            let Some(command) = input.read_byte(deadline)? else {
                break;
            };

            match command {
                SE => {
                    sb_mode = false;
                    if sb.first().copied() == Some(TTYPE) && sb.len() >= 2 {
                        info.term = Some(String::from_utf8_lossy(&sb[2..]).into_owned());
                        got_ttype = true;
                        deadline = Instant::now() + Duration::from_secs(2);
                    } else if sb.first().copied() == Some(NAWS) && sb.len() >= 5 {
                        info.width = Some(u16::from_be_bytes([sb[1], sb[2]]) as i32);
                        info.height = Some(u16::from_be_bytes([sb[3], sb[4]]) as i32);
                        got_naws = true;
                        deadline = Instant::now() + Duration::from_secs(2);
                    }
                }
                NOP => {
                    state.send_command(out, NOP, 0)?;
                    out.flush()?;
                }
                WILL | WONT => {
                    let Some(opt) = input.read_byte(deadline)? else {
                        break;
                    };
                    if state.willack[opt as usize] == 0 {
                        state.willack[opt as usize] = WONT;
                    }
                    state.send_command(out, state.willack[opt as usize], opt)?;
                    out.flush()?;
                    if command == WILL && opt == TTYPE {
                        out.write_all(&[IAC, SB, TTYPE, SEND, IAC, SE])?;
                        out.flush()?;
                    }
                }
                DO | DONT => {
                    let Some(opt) = input.read_byte(deadline)? else {
                        break;
                    };
                    if state.options[opt as usize] == 0 {
                        state.options[opt as usize] = DONT;
                    }
                    state.send_command(out, state.options[opt as usize], opt)?;
                    out.flush()?;
                }
                SB => {
                    sb_mode = true;
                    sb.clear();
                }
                IAC => {
                    // IAC IAC signals end of negotiation; bail out early
                    got_ttype = true;
                    got_naws = true;
                }
                _ => {}
            }
        } else if sb_mode && sb.len() < 1023 {
            sb.push(byte);
        }
    }

    Ok(info)
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

fn terminal_size() -> (i32, i32) {
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

fn detect_terminal_type(term: Option<&str>, terminal_width: i32) -> u8 {
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

fn parse_args(args: &[String], config: &mut Config) {
    let mut i = 1usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            break;
        }

        if let Some(long) = arg.strip_prefix("--") {
            let (name, value) = long.split_once('=').map_or((long, None), |(name, value)| {
                (name, Some(value.to_string()))
            });
            match name {
                "help" => {
                    usage(&args[0]);
                    process::exit(0);
                }
                "telnet" => config.telnet = true,
                "intro" => config.show_intro = true,
                "skip-intro" => config.skip_intro = true,
                "no-counter" => config.show_counter = false,
                "no-title" => config.set_title = false,
                "no-clear" => config.clear_screen = false,
                "delay" | "frames" | "min-rows" | "max-rows" | "min-cols" | "max-cols"
                | "width" | "height" => {
                    let value = value.unwrap_or_else(|| {
                        i += 1;
                        args.get(i).cloned().unwrap_or_default()
                    });
                    apply_option(config, long_to_short(name), &value);
                }
                _ => {}
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let bytes = arg.as_bytes();
            let mut pos = 1usize;
            while pos < bytes.len() {
                let opt = bytes[pos] as char;
                pos += 1;
                if option_requires_value(opt) {
                    let value = if pos < bytes.len() {
                        String::from_utf8_lossy(&bytes[pos..]).into_owned()
                    } else {
                        i += 1;
                        args.get(i).cloned().unwrap_or_default()
                    };
                    apply_option(config, opt, &value);
                    break;
                }
                apply_flag(config, opt, &args[0]);
            }
        }

        i += 1;
    }
}

fn apply_flag(config: &mut Config, opt: char, program: &str) {
    match opt {
        'e' => config.clear_screen = false,
        's' => config.set_title = false,
        'i' => config.show_intro = true,
        'I' => config.skip_intro = true,
        't' => config.telnet = true,
        'h' => {
            usage(program);
            process::exit(0);
        }
        'n' => config.show_counter = false,
        _ => {}
    }
}

fn apply_option(config: &mut Config, opt: char, value: &str) {
    let parsed = value.parse::<i32>().unwrap_or(0);
    match opt {
        'd' => {
            if (10..=1000).contains(&parsed) {
                config.delay_ms = parsed as u64;
            }
        }
        'f' => config.frame_count = parsed.max(0) as u32,
        'r' => config.min_row = parsed,
        'R' => config.max_row = parsed,
        'c' => config.min_col = parsed,
        'C' => config.max_col = parsed,
        'W' => {
            config.min_col = (FRAME_WIDTH as i32 - parsed) / 2;
            config.max_col = (FRAME_WIDTH as i32 + parsed) / 2;
        }
        'H' => {
            config.min_row = (FRAME_HEIGHT as i32 - parsed) / 2;
            config.max_row = (FRAME_HEIGHT as i32 + parsed) / 2;
        }
        _ => {}
    }
}

fn long_to_short(name: &str) -> char {
    match name {
        "delay" => 'd',
        "frames" => 'f',
        "min-rows" => 'r',
        "max-rows" => 'R',
        "min-cols" => 'c',
        "max-cols" => 'C',
        "width" => 'W',
        "height" => 'H',
        _ => '\0',
    }
}

fn option_requires_value(opt: char) -> bool {
    matches!(opt, 'd' | 'f' | 'r' | 'R' | 'c' | 'C' | 'W' | 'H')
}

fn usage(program: &str) {
    println!(
        "Terminal Nyancat\n\
         \n\
         usage: {program} [-hitn] [-f \x1b[3mframes\x1b[0m]\n\
         \n\
          -i --intro      \x1b[3mShow the introduction / about information at startup.\x1b[0m\n\
          -t --telnet     \x1b[3mTelnet mode.\x1b[0m\n\
          -n --no-counter \x1b[3mDo not display the timer\x1b[0m\n\
          -s --no-title   \x1b[3mDo not set the titlebar text\x1b[0m\n\
          -e --no-clear   \x1b[3mDo not clear the display between frames\x1b[0m\n\
          -d --delay      \x1b[3mDelay image rendering by anywhere between 10ms and 1000ms\n\
          -f --frames     \x1b[3mDisplay the requested number of frames, then quit\x1b[0m\n\
          -r --min-rows   \x1b[3mCrop the animation from the top\x1b[0m\n\
          -R --max-rows   \x1b[3mCrop the animation from the bottom\x1b[0m\n\
          -c --min-cols   \x1b[3mCrop the animation from the left\x1b[0m\n\
          -C --max-cols   \x1b[3mCrop the animation from the right\x1b[0m\n\
          -W --width      \x1b[3mCrop the animation to the given width\x1b[0m\n\
          -H --height     \x1b[3mCrop the animation to the given height\x1b[0m\n\
          -h --help       \x1b[3mShow this help message.\x1b[0m"
    );
}

fn install_signal_handlers() {
    unsafe {
        sys::signal(sys::SIGINT, handle_exit_signal as *const () as usize);
        sys::signal(sys::SIGPIPE, handle_exit_signal as *const () as usize);
        sys::signal(sys::SIGWINCH, handle_resize_signal as *const () as usize);
    }
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

fn finish(clear_screen: bool) -> ! {
    let mut stdout = io::stdout().lock();
    if clear_screen {
        let _ = stdout.write_all(b"\x1b[?25h\x1b[0m\x1b[H\x1b[2J");
    } else {
        let _ = stdout.write_all(b"\x1b[0m\n");
    }
    let _ = stdout.flush();
    process::exit(0);
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

    #[test]
    fn width_and_height_options_center_crop() {
        let mut config = Config::default();
        apply_option(&mut config, 'W', "40");
        apply_option(&mut config, 'H', "24");
        assert_eq!((config.min_col, config.max_col), (12, 52));
        assert_eq!((config.min_row, config.max_row), (20, 44));
    }
}
