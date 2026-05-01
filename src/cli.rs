use crate::animation::{FRAME_HEIGHT, FRAME_WIDTH};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CropBounds {
    pub(crate) rows: AxisCrop,
    pub(crate) cols: AxisCrop,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct AxisCrop {
    min: Option<i32>,
    max: Option<i32>,
}

impl AxisCrop {
    const LEGACY_AUTO_VALUE: i32 = -1;

    fn centered(frame_size: i32, size: i32) -> Self {
        Self {
            min: Some((frame_size - size) / 2),
            max: Some((frame_size + size) / 2),
        }
    }

    fn set_min(&mut self, min: i32) {
        self.min = Some(min);
    }

    fn set_max(&mut self, max: i32) {
        self.max = Some(max);
    }

    pub(crate) fn min_or_default(self) -> i32 {
        self.min.unwrap_or(Self::LEGACY_AUTO_VALUE)
    }

    pub(crate) fn max_or_default(self) -> i32 {
        self.max.unwrap_or(Self::LEGACY_AUTO_VALUE)
    }

    pub(crate) fn is_automatic_range(self) -> bool {
        self.min_or_default() == self.max_or_default()
    }

    #[cfg(test)]
    fn as_pair(self) -> (i32, i32) {
        (self.min_or_default(), self.max_or_default())
    }
}

#[derive(Debug, Eq, PartialEq)]
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
    pub(crate) crop: CropBounds,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CliAction {
    Run(Config),
    Help { program: String },
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CliError {
    MissingValue {
        option: String,
    },
    InvalidValue {
        option: String,
        value: String,
    },
    ValueOutOfRange {
        option: String,
        value: i32,
        min: i32,
        max: i32,
    },
    UnknownOption {
        option: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { option } => write!(f, "missing value for {option}"),
            Self::InvalidValue { option, value } => {
                write!(f, "invalid value for {option}: {value}")
            }
            Self::ValueOutOfRange {
                option,
                value,
                min,
                max,
            } => write!(
                f,
                "value for {option} out of range: {value} (expected {min}-{max})"
            ),
            Self::UnknownOption { option } => write!(f, "unknown option: {option}"),
        }
    }
}

impl std::error::Error for CliError {}

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
            crop: CropBounds::default(),
        }
    }
}

pub(crate) fn parse_args(args: &[String]) -> Result<CliAction, CliError> {
    let program = args
        .first()
        .cloned()
        .unwrap_or_else(|| "nyancat".to_string());
    let mut config = Config::default();
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
                    return Ok(CliAction::Help {
                        program: program.clone(),
                    });
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
                    let option = format!("--{name}");
                    let value = match value {
                        Some(value) => value,
                        None => {
                            i += 1;
                            args.get(i).cloned().ok_or_else(|| CliError::MissingValue {
                                option: option.clone(),
                            })?
                        }
                    };
                    apply_option(&mut config, long_to_short(name), &option, &value)?;
                }
                _ => {
                    return Err(CliError::UnknownOption {
                        option: format!("--{name}"),
                    });
                }
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let bytes = arg.as_bytes();
            let mut pos = 1usize;
            while pos < bytes.len() {
                let opt = bytes[pos] as char;
                pos += 1;
                if option_requires_value(opt) {
                    let option = format!("-{opt}");
                    let value = if pos < bytes.len() {
                        String::from_utf8_lossy(&bytes[pos..]).into_owned()
                    } else {
                        i += 1;
                        args.get(i).cloned().ok_or_else(|| CliError::MissingValue {
                            option: option.clone(),
                        })?
                    };
                    apply_option(&mut config, opt, &option, &value)?;
                    break;
                }
                if let Some(action) = apply_flag(&mut config, opt, &program)? {
                    return Ok(action);
                }
            }
        }

        i += 1;
    }

    Ok(CliAction::Run(config))
}

fn apply_flag(
    config: &mut Config,
    opt: char,
    program: &str,
) -> Result<Option<CliAction>, CliError> {
    match opt {
        'e' => config.clear_screen = false,
        's' => config.set_title = false,
        'i' => config.show_intro = true,
        'I' => config.skip_intro = true,
        't' => config.telnet = true,
        'b' => config.benchmark = true,
        'T' => config.truecolor = true,
        'h' => {
            return Ok(Some(CliAction::Help {
                program: program.to_string(),
            }));
        }
        'n' => config.show_counter = false,
        _ => {
            return Err(CliError::UnknownOption {
                option: format!("-{opt}"),
            });
        }
    }

    Ok(None)
}

fn apply_option(config: &mut Config, opt: char, option: &str, value: &str) -> Result<(), CliError> {
    let parsed = value.parse::<i32>().map_err(|_| CliError::InvalidValue {
        option: option.to_string(),
        value: value.to_string(),
    })?;

    match opt {
        'd' => {
            if !(10..=1000).contains(&parsed) {
                return Err(CliError::ValueOutOfRange {
                    option: option.to_string(),
                    value: parsed,
                    min: 10,
                    max: 1000,
                });
            }
            config.delay_ms = parsed as u64;
        }
        'f' => config.frame_count = parsed.max(0) as u32,
        'r' => config.crop.rows.set_min(parsed),
        'R' => config.crop.rows.set_max(parsed),
        'c' => config.crop.cols.set_min(parsed),
        'C' => config.crop.cols.set_max(parsed),
        'W' => config.crop.cols = AxisCrop::centered(FRAME_WIDTH as i32, parsed),
        'H' => config.crop.rows = AxisCrop::centered(FRAME_HEIGHT as i32, parsed),
        _ => {
            return Err(CliError::UnknownOption {
                option: option.to_string(),
            });
        }
    }

    Ok(())
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

pub(crate) fn print_usage(program: &str) {
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
        apply_option(&mut config, 'W', "-W", "40").unwrap();
        apply_option(&mut config, 'H', "-H", "24").unwrap();
        assert_eq!(config.crop.cols.as_pair(), (12, 52));
        assert_eq!(config.crop.rows.as_pair(), (20, 44));
    }

    #[test]
    fn default_crop_is_automatic() {
        let config = Config::default();

        assert_eq!(config.crop.cols.as_pair(), (-1, -1));
        assert_eq!(config.crop.rows.as_pair(), (-1, -1));
        assert!(config.crop.cols.is_automatic_range());
        assert!(config.crop.rows.is_automatic_range());
    }

    #[test]
    fn parses_short_flags_and_options() {
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

        let CliAction::Run(config) = parse_args(&args).unwrap() else {
            panic!("expected run action");
        };

        assert!(config.truecolor);
        assert!(!config.show_counter);
        assert!(!config.set_title);
        assert!(!config.clear_screen);
        assert_eq!(config.delay_ms, 120);
        assert_eq!(config.frame_count, 3);
        assert!(config.skip_intro);
        assert_eq!(config.crop.cols.as_pair(), (12, 52));
        assert_eq!(config.crop.rows.as_pair(), (20, 44));
    }

    #[test]
    fn help_is_returned_without_exiting() {
        let args = vec!["nyancat".to_string(), "--help".to_string()];

        assert_eq!(
            parse_args(&args),
            Ok(CliAction::Help {
                program: "nyancat".to_string()
            })
        );
    }

    #[test]
    fn invalid_delay_is_reported() {
        let mut config = Config::default();

        assert_eq!(
            apply_option(&mut config, 'd', "-d", "9"),
            Err(CliError::ValueOutOfRange {
                option: "-d".to_string(),
                value: 9,
                min: 10,
                max: 1000,
            })
        );
        assert_eq!(
            apply_option(&mut config, 'd', "-d", "not-a-number"),
            Err(CliError::InvalidValue {
                option: "-d".to_string(),
                value: "not-a-number".to_string(),
            })
        );
    }

    #[test]
    fn missing_option_value_is_reported() {
        let args = vec!["nyancat".to_string(), "-d".to_string()];

        assert_eq!(
            parse_args(&args),
            Err(CliError::MissingValue {
                option: "-d".to_string()
            })
        );
    }

    #[test]
    fn unknown_option_is_reported() {
        let args = vec!["nyancat".to_string(), "--wat".to_string()];

        assert_eq!(
            parse_args(&args),
            Err(CliError::UnknownOption {
                option: "--wat".to_string()
            })
        );
    }
}
