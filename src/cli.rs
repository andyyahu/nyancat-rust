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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionId {
    NoClear,
    NoTitle,
    Intro,
    SkipIntro,
    Telnet,
    Benchmark,
    TrueColor,
    Help,
    NoCounter,
    Delay,
    Frames,
    MinRows,
    MaxRows,
    MinCols,
    MaxCols,
    Width,
    Height,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionArity {
    Flag,
    Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OptionSpec {
    id: OptionId,
    short: char,
    long: &'static str,
    arity: OptionArity,
    value_name: Option<&'static str>,
    description: &'static str,
}

impl OptionSpec {
    const fn flag(
        id: OptionId,
        short: char,
        long: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            short,
            long,
            arity: OptionArity::Flag,
            value_name: None,
            description,
        }
    }

    const fn value(
        id: OptionId,
        short: char,
        long: &'static str,
        value_name: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            short,
            long,
            arity: OptionArity::Value,
            value_name: Some(value_name),
            description,
        }
    }

    const fn takes_value(self) -> bool {
        matches!(self.arity, OptionArity::Value)
    }
}

const OPTION_SPECS: &[OptionSpec] = &[
    OptionSpec::flag(
        OptionId::Intro,
        'i',
        "intro",
        "Show the introduction / about information at startup.",
    ),
    OptionSpec::flag(
        OptionId::SkipIntro,
        'I',
        "skip-intro",
        "Skip the introduction in telnet mode.",
    ),
    OptionSpec::flag(OptionId::Telnet, 't', "telnet", "Telnet mode."),
    OptionSpec::flag(
        OptionId::TrueColor,
        'T',
        "truecolor",
        "Enable 24-bit TrueColor mode (high-definition rendering).",
    ),
    OptionSpec::flag(
        OptionId::NoCounter,
        'n',
        "no-counter",
        "Do not display the timer.",
    ),
    OptionSpec::flag(
        OptionId::NoTitle,
        's',
        "no-title",
        "Do not set the titlebar text.",
    ),
    OptionSpec::flag(
        OptionId::NoClear,
        'e',
        "no-clear",
        "Do not clear the display between frames.",
    ),
    OptionSpec::flag(
        OptionId::Benchmark,
        'b',
        "benchmark",
        "Run in benchmark mode (0ms delay). Warning: high CPU usage.",
    ),
    OptionSpec::flag(OptionId::Help, 'h', "help", "Show this help message."),
    OptionSpec::value(
        OptionId::Delay,
        'd',
        "delay",
        "ms",
        "Delay image rendering by anywhere between 10ms and 1000ms.",
    ),
    OptionSpec::value(
        OptionId::Frames,
        'f',
        "frames",
        "frames",
        "Display the requested number of frames, then quit.",
    ),
    OptionSpec::value(
        OptionId::MinRows,
        'r',
        "min-rows",
        "row",
        "Crop the animation from the top.",
    ),
    OptionSpec::value(
        OptionId::MaxRows,
        'R',
        "max-rows",
        "row",
        "Crop the animation from the bottom.",
    ),
    OptionSpec::value(
        OptionId::MinCols,
        'c',
        "min-cols",
        "col",
        "Crop the animation from the left.",
    ),
    OptionSpec::value(
        OptionId::MaxCols,
        'C',
        "max-cols",
        "col",
        "Crop the animation from the right.",
    ),
    OptionSpec::value(
        OptionId::Width,
        'W',
        "width",
        "width",
        "Crop the animation to the given width.",
    ),
    OptionSpec::value(
        OptionId::Height,
        'H',
        "height",
        "height",
        "Crop the animation to the given height.",
    ),
];

fn option_by_long(long: &str) -> Option<OptionSpec> {
    OPTION_SPECS.iter().copied().find(|spec| spec.long == long)
}

fn option_by_short(short: char) -> Option<OptionSpec> {
    OPTION_SPECS
        .iter()
        .copied()
        .find(|spec| spec.short == short)
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
            let Some(spec) = option_by_long(name) else {
                return Err(CliError::UnknownOption {
                    option: format!("--{name}"),
                });
            };

            if spec.takes_value() {
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
                apply_value_option(&mut config, spec, &option, &value)?;
            } else if let Some(action) =
                apply_flag(&mut config, spec, &format!("--{name}"), &program)?
            {
                return Ok(action);
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let bytes = arg.as_bytes();
            let mut pos = 1usize;
            while pos < bytes.len() {
                let opt = bytes[pos] as char;
                pos += 1;

                let Some(spec) = option_by_short(opt) else {
                    return Err(CliError::UnknownOption {
                        option: format!("-{opt}"),
                    });
                };

                if spec.takes_value() {
                    let option = format!("-{opt}");
                    let value = if pos < bytes.len() {
                        String::from_utf8_lossy(&bytes[pos..]).into_owned()
                    } else {
                        i += 1;
                        args.get(i).cloned().ok_or_else(|| CliError::MissingValue {
                            option: option.clone(),
                        })?
                    };
                    apply_value_option(&mut config, spec, &option, &value)?;
                    break;
                }
                if let Some(action) = apply_flag(&mut config, spec, &format!("-{opt}"), &program)? {
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
    spec: OptionSpec,
    option: &str,
    program: &str,
) -> Result<Option<CliAction>, CliError> {
    match spec.id {
        OptionId::NoClear => config.clear_screen = false,
        OptionId::NoTitle => config.set_title = false,
        OptionId::Intro => config.show_intro = true,
        OptionId::SkipIntro => config.skip_intro = true,
        OptionId::Telnet => config.telnet = true,
        OptionId::Benchmark => config.benchmark = true,
        OptionId::TrueColor => config.truecolor = true,
        OptionId::Help => {
            return Ok(Some(CliAction::Help {
                program: program.to_string(),
            }));
        }
        OptionId::NoCounter => config.show_counter = false,
        _ => {
            return Err(CliError::UnknownOption {
                option: option.to_string(),
            });
        }
    }

    Ok(None)
}

fn apply_value_option(
    config: &mut Config,
    spec: OptionSpec,
    option: &str,
    value: &str,
) -> Result<(), CliError> {
    let parsed = value.parse::<i32>().map_err(|_| CliError::InvalidValue {
        option: option.to_string(),
        value: value.to_string(),
    })?;

    match spec.id {
        OptionId::Delay => {
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
        OptionId::Frames => config.frame_count = parsed.max(0) as u32,
        OptionId::MinRows => config.crop.rows.set_min(parsed),
        OptionId::MaxRows => config.crop.rows.set_max(parsed),
        OptionId::MinCols => config.crop.cols.set_min(parsed),
        OptionId::MaxCols => config.crop.cols.set_max(parsed),
        OptionId::Width => config.crop.cols = AxisCrop::centered(FRAME_WIDTH as i32, parsed),
        OptionId::Height => config.crop.rows = AxisCrop::centered(FRAME_HEIGHT as i32, parsed),
        _ => {
            return Err(CliError::UnknownOption {
                option: option.to_string(),
            });
        }
    }

    Ok(())
}

pub(crate) fn usage_text(program: &str) -> String {
    let mut usage = format!("Terminal Nyancat\n\nusage: {program} [options]\n\n");

    for spec in OPTION_SPECS {
        let option = match spec.value_name {
            Some(value_name) => format!("-{}, --{} <{}>", spec.short, spec.long, value_name),
            None => format!("-{}, --{}", spec.short, spec.long),
        };
        usage.push_str(&format!(
            "  {option:<25}\x1b[3m{}\x1b[0m\n",
            spec.description
        ));
    }

    usage
}

pub(crate) fn print_usage(program: &str) {
    print!("{}", usage_text(program));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_spec(short: char) -> OptionSpec {
        option_by_short(short).unwrap()
    }

    #[test]
    fn width_and_height_options_center_crop() {
        let mut config = Config::default();
        apply_value_option(&mut config, value_spec('W'), "-W", "40").unwrap();
        apply_value_option(&mut config, value_spec('H'), "-H", "24").unwrap();
        assert_eq!(config.crop.cols.as_pair(), (12, 52));
        assert_eq!(config.crop.rows.as_pair(), (20, 44));
    }

    #[test]
    fn option_specs_resolve_long_and_short_forms() {
        let frames = option_by_long("frames").unwrap();

        assert_eq!(frames.short, 'f');
        assert_eq!(option_by_short('f'), Some(frames));
        assert!(frames.takes_value());
        assert!(!option_by_long("telnet").unwrap().takes_value());
        assert_eq!(option_by_long("wat"), None);
        assert_eq!(option_by_short('?'), None);
    }

    #[test]
    fn option_specs_are_unique() {
        for (index, spec) in OPTION_SPECS.iter().enumerate() {
            for other in &OPTION_SPECS[index + 1..] {
                assert_ne!(spec.short, other.short);
                assert_ne!(spec.long, other.long);
            }
        }
    }

    #[test]
    fn usage_text_lists_every_option_spec() {
        let usage = usage_text("nyancat");

        for spec in OPTION_SPECS {
            assert!(usage.contains(&format!("-{}, --{}", spec.short, spec.long)));
            assert!(usage.contains(spec.description));
            if let Some(value_name) = spec.value_name {
                assert!(usage.contains(&format!("<{value_name}>")));
            }
        }
    }

    #[test]
    fn usage_text_resets_italic_descriptions() {
        let usage = usage_text("nyancat");

        for line in usage.lines().filter(|line| line.contains("\x1b[3m")) {
            assert!(line.ends_with("\x1b[0m"));
        }
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
        let delay = value_spec('d');

        assert_eq!(
            apply_value_option(&mut config, delay, "-d", "9"),
            Err(CliError::ValueOutOfRange {
                option: "-d".to_string(),
                value: 9,
                min: 10,
                max: 1000,
            })
        );
        assert_eq!(
            apply_value_option(&mut config, delay, "-d", "not-a-number"),
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
