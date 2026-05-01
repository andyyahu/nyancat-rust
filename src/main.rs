mod animation;
mod cli;
mod render;
mod runtime;
mod sys;
mod telnet;
mod terminal;

use cli::{CliAction, parse_args, print_usage};
use render::{Palette, RenderState, RunOutcome, run};
use runtime::{
    clear_screen_on_exit, install_signal_handlers, restore_terminal, set_clear_screen_on_exit,
};
use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use telnet::negotiate_telnet;
use terminal::{TerminalType, detect_terminal_type, terminal_size};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut config = match parse_args(&args) {
        Ok(CliAction::Run(config)) => config,
        Ok(CliAction::Help { program }) => {
            print_usage(&program);
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            let program = args.first().map_or("nyancat", String::as_str);
            let _ = writeln!(io::stderr(), "nyancat: {error}");
            let _ = writeln!(io::stderr(), "Try '{program} --help' for usage.");
            return ExitCode::FAILURE;
        }
    };

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
            Err(_) => return restore_and_succeed(config.clear_screen),
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

    let mut terminal_type = detect_terminal_type(term.as_deref(), terminal_width);
    if config.truecolor {
        terminal_type = TerminalType::TrueColor;
    }
    if terminal_type == TerminalType::Vt100Ascii {
        terminal_width = 40;
    }

    let palette = Palette::new(terminal_type);
    let mut state = RenderState::new(&config, terminal_width, terminal_height);
    state.finalize_auto_crop();

    match run(config, state, palette) {
        Ok(RunOutcome::FrameLimitReached { clear_screen }) => restore_and_succeed(clear_screen),
        Err(error) => {
            if error.kind() == io::ErrorKind::BrokenPipe {
                return restore_and_succeed(clear_screen_on_exit());
            }
            let _ = writeln!(io::stderr(), "nyancat: {error}");
            restore_and_succeed(clear_screen_on_exit())
        }
    }
}

fn restore_and_succeed(clear_screen: bool) -> ExitCode {
    let _ = restore_terminal(clear_screen);
    ExitCode::SUCCESS
}
