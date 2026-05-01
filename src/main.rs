mod animation;
mod cli;
mod render;
mod runtime;
mod sys;
mod telnet;
mod terminal;

use cli::{Config, parse_args};
use render::{Palette, RenderState, run};
use runtime::{clear_screen_on_exit, finish, install_signal_handlers, set_clear_screen_on_exit};
use std::env;
use std::io::{self, Write};
use telnet::negotiate_telnet;
use terminal::{detect_terminal_type, terminal_size};

fn main() {
    let mut config = Config::default();
    let args: Vec<String> = env::args().collect();
    parse_args(&args, &mut config);

    if config.benchmark {
        config.delay_ms = 0;
        let _ = writeln!(
            io::stderr(),
            "\x1b[1;33mWARNING:\x1b[0m Benchmark mode enabled. Delay set to 0ms."
        );
    }

    if config.telnet && !config.skip_intro {
        config.show_intro = true;
    }

    set_clear_screen_on_exit(config.clear_screen);
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

    let mut ttype = detect_terminal_type(term.as_deref(), terminal_width);
    if config.truecolor {
        ttype = 8;
    }
    if ttype == 7 {
        terminal_width = 40;
    }

    let palette = Palette::new(ttype);
    let mut state = RenderState::new(&config, terminal_width, terminal_height);
    state.finalize_auto_crop();

    if let Err(error) = run(config, state, palette) {
        if error.kind() == io::ErrorKind::BrokenPipe {
            finish(clear_screen_on_exit());
        }
        let _ = writeln!(io::stderr(), "nyancat: {error}");
        finish(clear_screen_on_exit());
    }
}
