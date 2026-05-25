use crate::sys;
use crate::terminal::TerminalSize;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const IAC: u8 = 255;
const SEND: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TelnetCommand {
    Se,
    Nop,
    Sb,
    Will,
    Wont,
    Do,
    Dont,
    Iac,
    Unknown(u8),
}

impl TelnetCommand {
    fn from_byte(byte: u8) -> Self {
        match byte {
            240 => Self::Se,
            241 => Self::Nop,
            250 => Self::Sb,
            251 => Self::Will,
            252 => Self::Wont,
            253 => Self::Do,
            254 => Self::Dont,
            255 => Self::Iac,
            byte => Self::Unknown(byte),
        }
    }

    const fn raw(self) -> u8 {
        match self {
            Self::Se => 240,
            Self::Nop => 241,
            Self::Sb => 250,
            Self::Will => 251,
            Self::Wont => 252,
            Self::Do => 253,
            Self::Dont => 254,
            Self::Iac => 255,
            Self::Unknown(byte) => byte,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TelnetOption(u8);

impl TelnetOption {
    const ECHO: Self = Self(1);
    const SGA: Self = Self(3);
    const TTYPE: Self = Self(24);
    const NAWS: Self = Self(31);
    const LINEMODE: Self = Self(34);
    const NEW_ENVIRON: Self = Self(39);

    const fn new(byte: u8) -> Self {
        Self(byte)
    }

    const fn raw(self) -> u8 {
        self.0
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

trait ByteSource {
    fn read_byte(&mut self, deadline: Instant) -> io::Result<Option<u8>>;
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
}

impl ByteSource for TimeoutReader {
    fn read_byte(&mut self, deadline: Instant) -> io::Result<Option<u8>> {
        if self.head < self.tail {
            let byte = self.buffer[self.head];
            self.head += 1;
            return Ok(Some(byte));
        }

        loop {
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }

            let timeout = sys::PollTimeout::from_duration(deadline.saturating_duration_since(now));

            match sys::stdin_readiness(timeout)? {
                sys::PollReadiness::Ready => match sys::read_stdin(&mut self.buffer)? {
                    sys::StdinRead::Bytes(bytes_read) => {
                        self.head = 1;
                        self.tail = bytes_read;
                        return Ok(Some(self.buffer[0]));
                    }
                    sys::StdinRead::Eof => return Ok(None),
                    sys::StdinRead::Interrupted => {}
                },
                sys::PollReadiness::Timeout => return Ok(None),
                sys::PollReadiness::Interrupted => {}
            }
        }
    }
}

struct TelnetState {
    options: [Option<TelnetCommand>; 256],
    willack: [Option<TelnetCommand>; 256],
    do_set: [Option<TelnetCommand>; 256],
    will_set: [Option<TelnetCommand>; 256],
}

impl TelnetState {
    fn new() -> Self {
        let mut state = Self {
            options: [None; 256],
            willack: [None; 256],
            do_set: [None; 256],
            will_set: [None; 256],
        };

        state.options[TelnetOption::ECHO.index()] = Some(TelnetCommand::Wont);
        state.options[TelnetOption::SGA.index()] = Some(TelnetCommand::Will);
        state.options[TelnetOption::NEW_ENVIRON.index()] = Some(TelnetCommand::Wont);
        state.willack[TelnetOption::ECHO.index()] = Some(TelnetCommand::Do);
        state.willack[TelnetOption::SGA.index()] = Some(TelnetCommand::Do);
        state.willack[TelnetOption::NAWS.index()] = Some(TelnetCommand::Do);
        state.willack[TelnetOption::TTYPE.index()] = Some(TelnetCommand::Do);
        state.willack[TelnetOption::LINEMODE.index()] = Some(TelnetCommand::Dont);
        state.willack[TelnetOption::NEW_ENVIRON.index()] = Some(TelnetCommand::Do);

        state
    }

    fn push_command(&mut self, out: &mut Vec<u8>, command: TelnetCommand) {
        out.extend_from_slice(&[IAC, command.raw()]);
    }

    fn push_option_command(
        &mut self,
        out: &mut Vec<u8>,
        command: TelnetCommand,
        option: TelnetOption,
    ) {
        match command {
            TelnetCommand::Do | TelnetCommand::Dont => {
                let current = self.do_set[option.index()];
                if current != Some(command) {
                    self.do_set[option.index()] = Some(command);
                    out.extend_from_slice(&[IAC, command.raw(), option.raw()]);
                }
            }
            TelnetCommand::Will | TelnetCommand::Wont => {
                let current = self.will_set[option.index()];
                if current != Some(command) {
                    self.will_set[option.index()] = Some(command);
                    out.extend_from_slice(&[IAC, command.raw(), option.raw()]);
                }
            }
            _ => self.push_command(out, command),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct TelnetInfo {
    pub(crate) term: Option<String>,
    pub(crate) size: Option<TerminalSize>,
}

#[derive(Debug, Eq, PartialEq)]
enum Subnegotiation {
    TerminalType(String),
    WindowSize(TerminalSize),
}

fn parse_subnegotiation(bytes: &[u8]) -> Option<Subnegotiation> {
    match bytes.first().copied().map(TelnetOption::new) {
        Some(TelnetOption::TTYPE) if bytes.len() >= 2 => Some(Subnegotiation::TerminalType(
            String::from_utf8_lossy(&bytes[2..]).into_owned(),
        )),
        Some(TelnetOption::NAWS) if bytes.len() >= 5 => TerminalSize::try_new(
            u16::from_be_bytes([bytes[1], bytes[2]]) as i32,
            u16::from_be_bytes([bytes[3], bytes[4]]) as i32,
        )
        .map(Subnegotiation::WindowSize),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TelnetParserState {
    Data,
    DataIac,
    CommandOption {
        command: TelnetCommand,
        in_subnegotiation: bool,
    },
    Subnegotiation,
    SubnegotiationIac,
}

#[derive(Debug, Eq, PartialEq)]
enum TelnetEvent {
    Command(TelnetCommand),
    Negotiation {
        command: TelnetCommand,
        option: TelnetOption,
    },
    Subnegotiation(Vec<u8>),
}

struct TelnetParser {
    state: TelnetParserState,
    sb: Vec<u8>,
}

impl TelnetParser {
    fn new() -> Self {
        Self {
            state: TelnetParserState::Data,
            sb: Vec::with_capacity(1024),
        }
    }

    fn push(&mut self, byte: u8) -> Option<TelnetEvent> {
        match self.state {
            TelnetParserState::Data => {
                if byte == IAC {
                    self.state = TelnetParserState::DataIac;
                }
                None
            }
            TelnetParserState::DataIac => self.handle_iac(byte, false),
            TelnetParserState::CommandOption {
                command,
                in_subnegotiation,
            } => {
                self.state = if in_subnegotiation {
                    TelnetParserState::Subnegotiation
                } else {
                    TelnetParserState::Data
                };
                Some(TelnetEvent::Negotiation {
                    command,
                    option: TelnetOption::new(byte),
                })
            }
            TelnetParserState::Subnegotiation => {
                if byte == IAC {
                    self.state = TelnetParserState::SubnegotiationIac;
                } else if self.sb.len() < 1023 {
                    self.sb.push(byte);
                }
                None
            }
            TelnetParserState::SubnegotiationIac => self.handle_iac(byte, true),
        }
    }

    fn handle_iac(&mut self, byte: u8, in_subnegotiation: bool) -> Option<TelnetEvent> {
        let command = TelnetCommand::from_byte(byte);
        match command {
            TelnetCommand::Se => {
                self.state = TelnetParserState::Data;
                Some(TelnetEvent::Subnegotiation(self.sb.clone()))
            }
            TelnetCommand::Nop => {
                self.state = if in_subnegotiation {
                    TelnetParserState::Subnegotiation
                } else {
                    TelnetParserState::Data
                };
                Some(TelnetEvent::Command(TelnetCommand::Nop))
            }
            TelnetCommand::Will | TelnetCommand::Wont | TelnetCommand::Do | TelnetCommand::Dont => {
                self.state = TelnetParserState::CommandOption {
                    command,
                    in_subnegotiation,
                };
                None
            }
            TelnetCommand::Sb => {
                self.state = TelnetParserState::Subnegotiation;
                self.sb.clear();
                None
            }
            TelnetCommand::Iac => {
                if in_subnegotiation {
                    self.state = TelnetParserState::Subnegotiation;
                    if self.sb.len() < 1023 {
                        self.sb.push(IAC);
                    }
                } else {
                    self.state = TelnetParserState::Data;
                }
                None
            }
            _ => {
                self.state = if in_subnegotiation {
                    TelnetParserState::Subnegotiation
                } else {
                    TelnetParserState::Data
                };
                None
            }
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
struct NegotiationStep {
    output: Vec<u8>,
    extend_deadline: bool,
}

struct TelnetNegotiation {
    state: TelnetState,
    info: TelnetInfo,
    got_ttype: bool,
    got_naws: bool,
}

impl TelnetNegotiation {
    fn new() -> Self {
        Self {
            state: TelnetState::new(),
            info: TelnetInfo::default(),
            got_ttype: false,
            got_naws: false,
        }
    }

    fn initial_output(&mut self) -> Vec<u8> {
        let mut output = Vec::new();

        for option in 0..=255u8 {
            let option = TelnetOption::new(option);
            let cmd_opt = self.state.options[option.index()];
            if let Some(command) = cmd_opt {
                self.state.push_option_command(&mut output, command, option);
            }
            let cmd_willack = self.state.willack[option.index()];
            if let Some(command) = cmd_willack {
                self.state.push_option_command(&mut output, command, option);
            }
        }

        output
    }

    fn is_complete(&self) -> bool {
        self.got_ttype && self.got_naws
    }

    fn into_info(self) -> TelnetInfo {
        self.info
    }

    fn handle_event(&mut self, event: TelnetEvent) -> NegotiationStep {
        let mut step = NegotiationStep::default();

        match event {
            TelnetEvent::Command(TelnetCommand::Nop) => {
                self.state
                    .push_command(&mut step.output, TelnetCommand::Nop);
            }
            TelnetEvent::Command(_) => {}
            TelnetEvent::Negotiation { command, option } => match command {
                TelnetCommand::Will | TelnetCommand::Wont => {
                    self.handle_will_wont(command, option, &mut step.output)
                }
                TelnetCommand::Do | TelnetCommand::Dont => {
                    self.handle_do_dont(option, &mut step.output)
                }
                _ => {}
            },
            TelnetEvent::Subnegotiation(bytes) => {
                if self.handle_subnegotiation(&bytes) {
                    step.extend_deadline = true;
                }
            }
        }

        step
    }

    fn handle_will_wont(
        &mut self,
        command: TelnetCommand,
        option: TelnetOption,
        output: &mut Vec<u8>,
    ) {
        if self.state.willack[option.index()].is_none() {
            self.state.willack[option.index()] = Some(TelnetCommand::Wont);
        }
        if let Some(response) = self.state.willack[option.index()] {
            self.state.push_option_command(output, response, option);
        }

        if command == TelnetCommand::Will && option == TelnetOption::TTYPE {
            output.extend_from_slice(&[
                IAC,
                TelnetCommand::Sb.raw(),
                TelnetOption::TTYPE.raw(),
                SEND,
                IAC,
                TelnetCommand::Se.raw(),
            ]);
        }
    }

    fn handle_do_dont(&mut self, option: TelnetOption, output: &mut Vec<u8>) {
        if self.state.options[option.index()].is_none() {
            self.state.options[option.index()] = Some(TelnetCommand::Dont);
        }
        if let Some(response) = self.state.options[option.index()] {
            self.state.push_option_command(output, response, option);
        }
    }

    fn handle_subnegotiation(&mut self, bytes: &[u8]) -> bool {
        match parse_subnegotiation(bytes) {
            Some(Subnegotiation::TerminalType(term)) => {
                self.info.term = Some(term);
                self.got_ttype = true;
                true
            }
            Some(Subnegotiation::WindowSize(size)) => {
                self.info.size = Some(size);
                self.got_naws = true;
                true
            }
            None => false,
        }
    }
}

pub(crate) fn negotiate_telnet(out: &mut impl Write) -> io::Result<TelnetInfo> {
    let mut input = TimeoutReader::new();
    negotiate_telnet_with_source(out, &mut input)
}

fn negotiate_telnet_with_source(
    out: &mut impl Write,
    input: &mut impl ByteSource,
) -> io::Result<TelnetInfo> {
    let mut negotiation = TelnetNegotiation::new();
    out.write_all(&negotiation.initial_output())?;
    out.flush()?;

    let mut parser = TelnetParser::new();
    let mut deadline = Instant::now() + Duration::from_secs(1);

    while !negotiation.is_complete() {
        let Some(byte) = input.read_byte(deadline)? else {
            break;
        };

        let Some(event) = parser.push(byte) else {
            continue;
        };
        let step = negotiation.handle_event(event);

        if !step.output.is_empty() {
            out.write_all(&step.output)?;
            out.flush()?;
        }
        if step.extend_deadline {
            deadline = Instant::now() + Duration::from_secs(2);
        }
    }

    Ok(negotiation.into_info())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser_events(bytes: &[u8]) -> Vec<TelnetEvent> {
        let mut parser = TelnetParser::new();
        bytes.iter().filter_map(|byte| parser.push(*byte)).collect()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn command(command: TelnetCommand) -> u8 {
        command.raw()
    }

    fn option(option: TelnetOption) -> u8 {
        option.raw()
    }

    fn option_command(command: TelnetCommand, option: TelnetOption) -> [u8; 3] {
        [IAC, command.raw(), option.raw()]
    }

    fn terminal_type_send() -> [u8; 6] {
        [
            IAC,
            command(TelnetCommand::Sb),
            option(TelnetOption::TTYPE),
            SEND,
            IAC,
            command(TelnetCommand::Se),
        ]
    }

    struct ScriptedByteSource {
        bytes: Vec<u8>,
        position: usize,
    }

    impl ScriptedByteSource {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes, position: 0 }
        }
    }

    impl ByteSource for ScriptedByteSource {
        fn read_byte(&mut self, _deadline: Instant) -> io::Result<Option<u8>> {
            let Some(byte) = self.bytes.get(self.position).copied() else {
                return Ok(None);
            };
            self.position += 1;
            Ok(Some(byte))
        }
    }

    #[test]
    fn parses_terminal_type_subnegotiation() {
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::TTYPE), 0, b'x', b't', b'e', b'r', b'm']),
            Some(Subnegotiation::TerminalType("xterm".to_string()))
        );
    }

    #[test]
    fn parses_window_size_subnegotiation() {
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::NAWS), 0, 120, 0, 40]),
            Some(Subnegotiation::WindowSize(TerminalSize::new(120, 40)))
        );
    }

    #[test]
    fn ignores_zero_window_size_subnegotiation() {
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::NAWS), 0, 0, 0, 40]),
            None
        );
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::NAWS), 0, 120, 0, 0]),
            None
        );
    }

    #[test]
    fn ignores_incomplete_or_unknown_subnegotiation() {
        assert_eq!(parse_subnegotiation(&[]), None);
        assert_eq!(parse_subnegotiation(&[option(TelnetOption::TTYPE)]), None);
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::NAWS), 0, 80, 0]),
            None
        );
        assert_eq!(
            parse_subnegotiation(&[option(TelnetOption::NEW_ENVIRON), 0]),
            None
        );
    }

    #[test]
    fn parser_emits_negotiation_commands() {
        assert_eq!(
            parser_events(&[
                IAC,
                command(TelnetCommand::Will),
                option(TelnetOption::TTYPE)
            ]),
            vec![TelnetEvent::Negotiation {
                command: TelnetCommand::Will,
                option: TelnetOption::TTYPE,
            }]
        );
    }

    #[test]
    fn parser_emits_subnegotiation_payloads() {
        assert_eq!(
            parser_events(&[
                IAC,
                command(TelnetCommand::Sb),
                option(TelnetOption::NAWS),
                0,
                80,
                0,
                24,
                IAC,
                command(TelnetCommand::Se)
            ]),
            vec![TelnetEvent::Subnegotiation(vec![
                option(TelnetOption::NAWS),
                0,
                80,
                0,
                24
            ])]
        );
    }

    #[test]
    fn parser_keeps_subnegotiation_mode_after_embedded_commands() {
        assert_eq!(
            parser_events(&[
                IAC,
                command(TelnetCommand::Sb),
                b'a',
                IAC,
                command(TelnetCommand::Nop),
                b'b',
                IAC,
                command(TelnetCommand::Se)
            ]),
            vec![
                TelnetEvent::Command(TelnetCommand::Nop),
                TelnetEvent::Subnegotiation(vec![b'a', b'b']),
            ]
        );
    }

    #[test]
    fn parser_ignores_escaped_iac_data() {
        assert_eq!(parser_events(&[IAC, IAC]), Vec::<TelnetEvent>::new());
    }

    #[test]
    fn parser_keeps_escaped_iac_inside_subnegotiation() {
        assert_eq!(
            parser_events(&[
                IAC,
                command(TelnetCommand::Sb),
                option(TelnetOption::TTYPE),
                0,
                b'x',
                IAC,
                IAC,
                b'y',
                IAC,
                command(TelnetCommand::Se)
            ]),
            vec![TelnetEvent::Subnegotiation(vec![
                option(TelnetOption::TTYPE),
                0,
                b'x',
                IAC,
                b'y'
            ])]
        );
    }

    #[test]
    fn initial_output_advertises_supported_options() {
        let mut negotiation = TelnetNegotiation::new();
        let output = negotiation.initial_output();

        assert!(contains(
            &output,
            &option_command(TelnetCommand::Wont, TelnetOption::ECHO)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Will, TelnetOption::SGA)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::TTYPE)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::NAWS)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Dont, TelnetOption::LINEMODE)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Wont, TelnetOption::NEW_ENVIRON)
        ));
    }

    #[test]
    fn will_ttype_requests_terminal_type() {
        let mut negotiation = TelnetNegotiation::new();
        let _ = negotiation.initial_output();

        let step = negotiation.handle_event(TelnetEvent::Negotiation {
            command: TelnetCommand::Will,
            option: TelnetOption::TTYPE,
        });

        assert_eq!(step.output, terminal_type_send());
        assert!(!step.extend_deadline);
    }

    #[test]
    fn unknown_options_remain_pass_through_and_are_rejected() {
        let unknown = TelnetOption::new(200);
        let mut negotiation = TelnetNegotiation::new();
        let _ = negotiation.initial_output();

        let step = negotiation.handle_event(TelnetEvent::Negotiation {
            command: TelnetCommand::Will,
            option: unknown,
        });

        assert_eq!(step.output, option_command(TelnetCommand::Wont, unknown));

        let step = negotiation.handle_event(TelnetEvent::Negotiation {
            command: TelnetCommand::Do,
            option: unknown,
        });

        assert_eq!(step.output, option_command(TelnetCommand::Dont, unknown));
    }

    #[test]
    fn subnegotiation_updates_telnet_info() {
        let mut negotiation = TelnetNegotiation::new();

        let step = negotiation.handle_event(TelnetEvent::Subnegotiation(vec![
            option(TelnetOption::TTYPE),
            0,
            b'v',
            b't',
            b'1',
            b'0',
            b'0',
        ]));

        assert!(step.extend_deadline);
        assert_eq!(negotiation.info.term.as_deref(), Some("vt100"));
        assert!(!negotiation.is_complete());

        let step = negotiation.handle_event(TelnetEvent::Subnegotiation(vec![
            option(TelnetOption::NAWS),
            0,
            80,
            0,
            24,
        ]));

        assert!(step.extend_deadline);
        assert_eq!(negotiation.info.size, Some(TerminalSize::new(80, 24)));
        assert!(negotiation.is_complete());
    }

    #[test]
    fn negotiate_telnet_reads_scripted_terminal_info() {
        let mut input = ScriptedByteSource::new(vec![
            IAC,
            command(TelnetCommand::Will),
            option(TelnetOption::TTYPE),
            IAC,
            command(TelnetCommand::Sb),
            option(TelnetOption::TTYPE),
            0,
            b'x',
            b't',
            b'e',
            b'r',
            b'm',
            IAC,
            command(TelnetCommand::Se),
            IAC,
            command(TelnetCommand::Sb),
            option(TelnetOption::NAWS),
            0,
            100,
            0,
            40,
            IAC,
            command(TelnetCommand::Se),
        ]);
        let mut output = Vec::new();

        let info = negotiate_telnet_with_source(&mut output, &mut input).unwrap();

        assert_eq!(info.term.as_deref(), Some("xterm"));
        assert_eq!(info.size, Some(TerminalSize::new(100, 40)));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::TTYPE)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::NAWS)
        ));
        assert!(contains(&output, &terminal_type_send()));
    }

    #[test]
    fn negotiate_telnet_stops_when_scripted_input_ends() {
        let mut input = ScriptedByteSource::new(Vec::new());
        let mut output = Vec::new();

        let info = negotiate_telnet_with_source(&mut output, &mut input).unwrap();

        assert_eq!(info, TelnetInfo::default());
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::TTYPE)
        ));
        assert!(contains(
            &output,
            &option_command(TelnetCommand::Do, TelnetOption::NAWS)
        ));
    }
}
