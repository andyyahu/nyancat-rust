use crate::animation::{FRAME_HEIGHT, FRAME_WIDTH};
use std::process;

pub(crate) struct Config {
    pub(crate) telnet: bool,
    pub(crate) show_counter: bool,
    pub(crate) frame_count: u32,
    pub(crate) clear_screen: bool,
    pub(crate) set_title: bool,
    pub(crate) show_intro: bool,
    pub(crate) skip_intro: bool,
    pub(crate) delay_ms: u64,
    pub(crate) benchmark: bool,
    pub(crate) truecolor: bool,
    pub(crate) min_row: i32,
    pub(crate) max_row: i32,
    pub(crate) min_col: i32,
    pub(crate) max_col: i32,
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
            benchmark: false,
            truecolor: false,
            min_row: -1,
            max_row: -1,
            min_col: -1,
            max_col: -1,
        }
    }
}

pub(crate) fn parse_args(args: &[String], config: &mut Config) {
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
                "benchmark" => config.benchmark = true,
                "truecolor" => config.truecolor = true,
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
        'b' => config.benchmark = true,
        'T' => config.truecolor = true,
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
        'd' if (10..=1000).contains(&parsed) => {
            config.delay_ms = parsed as u64;
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
         usage: {program} [-hIitnTb] [-f \x1b[3mframes\x1b[0m]\n\
         \n\
          -i --intro      \x1b[3mShow the introduction / about information at startup.\x1b[0m\n\
          -I --skip-intro \x1b[3mSkip the introduction in telnet mode.\x1b[0m\n\
          -t --telnet     \x1b[3mTelnet mode.\x1b[0m\n\
          -T --truecolor  \x1b[3mEnable 24-bit TrueColor mode (high-definition rendering)\x1b[0m\n\
          -n --no-counter \x1b[3mDo not display the timer\x1b[0m\n\
          -s --no-title   \x1b[3mDo not set the titlebar text\x1b[0m\n\
          -e --no-clear   \x1b[3mDo not clear the display between frames\x1b[0m\n\
          -b --benchmark  \x1b[3mRun in benchmark mode (0ms delay). Warning: high CPU usage\x1b[0m\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_and_height_options_center_crop() {
        let mut config = Config::default();
        apply_option(&mut config, 'W', "40");
        apply_option(&mut config, 'H', "24");
        assert_eq!((config.min_col, config.max_col), (12, 52));
        assert_eq!((config.min_row, config.max_row), (20, 44));
    }

    #[test]
    fn parses_short_flags_and_options() {
        let mut config = Config::default();
        let args = vec![
            "nyancat".to_string(),
            "-Tnse".to_string(),
            "-d".to_string(),
            "120".to_string(),
            "-f3".to_string(),
            "--skip-intro".to_string(),
            "--width=40".to_string(),
            "--height".to_string(),
            "24".to_string(),
        ];

        parse_args(&args, &mut config);

        assert!(config.truecolor);
        assert!(!config.show_counter);
        assert!(!config.set_title);
        assert!(!config.clear_screen);
        assert_eq!(config.delay_ms, 120);
        assert_eq!(config.frame_count, 3);
        assert!(config.skip_intro);
        assert_eq!((config.min_col, config.max_col), (12, 52));
        assert_eq!((config.min_row, config.max_row), (20, 44));
    }

    #[test]
    fn invalid_delay_keeps_default() {
        let mut config = Config::default();

        apply_option(&mut config, 'd', "9");
        apply_option(&mut config, 'd', "1001");
        apply_option(&mut config, 'd', "not-a-number");

        assert_eq!(config.delay_ms, 90);
    }
}
