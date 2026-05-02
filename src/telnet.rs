use crate::sys;
use crate::terminal::TerminalSize;
use std::io::{self, Write};
use std::time::{Duration, Instant};

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

        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }

        let timeout = sys::PollTimeout::from_duration(deadline.saturating_duration_since(now));

        if sys::stdin_ready(timeout) {
            if let Some(bytes_read) = sys::read_stdin(&mut self.buffer)? {
                self.head = 1;
                self.tail = bytes_read;
                return Ok(Some(self.buffer[0]));
            }
        }

        Ok(None)
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

    fn push_command(&mut self, out: &mut Vec<u8>, cmd: u8, opt: u8) {
        match cmd {
            DO | DONT => {
                let current = self.do_set[opt as usize];
                if (cmd == DO && current != DO) || (cmd == DONT && current != DONT) {
                    self.do_set[opt as usize] = cmd;
                    out.extend_from_slice(&[IAC, cmd, opt]);
                }
            }
            WILL | WONT => {
                let current = self.will_set[opt as usize];
                if (cmd == WILL && current != WILL) || (cmd == WONT && current != WONT) {
                    self.will_set[opt as usize] = cmd;
                    out.extend_from_slice(&[IAC, cmd, opt]);
                }
            }
            _ => out.extend_from_slice(&[IAC, cmd]),
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
    match bytes.first().copied() {
        Some(TTYPE) if bytes.len() >= 2 => Some(Subnegotiation::TerminalType(
            String::from_utf8_lossy(&bytes[2..]).into_owned(),
        )),
        Some(NAWS) if bytes.len() >= 5 => Some(Subnegotiation::WindowSize(TerminalSize::new(
            u16::from_be_bytes([bytes[1], bytes[2]]) as i32,
            u16::from_be_bytes([bytes[3], bytes[4]]) as i32,
        ))),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TelnetParserState {
    Data,
    DataIac,
    CommandOption {
        command: u8,
        in_subnegotiation: bool,
    },
    Subnegotiation,
    SubnegotiationIac,
}

#[derive(Debug, Eq, PartialEq)]
enum TelnetEvent {
    Command(u8),
    Negotiation { command: u8, option: u8 },
    Subnegotiation(Vec<u8>),
    EndNegotiation,
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
                    option: byte,
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

    fn handle_iac(&mut self, command: u8, in_subnegotiation: bool) -> Option<TelnetEvent> {
        match command {
            SE => {
                self.state = TelnetParserState::Data;
                Some(TelnetEvent::Subnegotiation(self.sb.clone()))
            }
            NOP => {
                self.state = if in_subnegotiation {
                    TelnetParserState::Subnegotiation
                } else {
                    TelnetParserState::Data
                };
                Some(TelnetEvent::Command(NOP))
            }
            WILL | WONT | DO | DONT => {
                self.state = TelnetParserState::CommandOption {
                    command,
                    in_subnegotiation,
                };
                None
            }
            SB => {
                self.state = TelnetParserState::Subnegotiation;
                self.sb.clear();
                None
            }
            IAC => {
                self.state = TelnetParserState::Data;
                Some(TelnetEvent::EndNegotiation)
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
            let cmd_opt = self.state.options[option as usize];
            if cmd_opt != 0 {
                self.state.push_command(&mut output, cmd_opt, option);
            }
            let cmd_willack = self.state.willack[option as usize];
            if cmd_willack != 0 {
                self.state.push_command(&mut output, cmd_willack, option);
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
            TelnetEvent::Command(NOP) => {
                self.state.push_command(&mut step.output, NOP, 0);
            }
            TelnetEvent::Command(_) => {}
            TelnetEvent::Negotiation { command, option } => match command {
                WILL | WONT => self.handle_will_wont(command, option, &mut step.output),
                DO | DONT => self.handle_do_dont(option, &mut step.output),
                _ => {}
            },
            TelnetEvent::Subnegotiation(bytes) => {
                if self.handle_subnegotiation(&bytes) {
                    step.extend_deadline = true;
                }
            }
            TelnetEvent::EndNegotiation => {
                self.got_ttype = true;
                self.got_naws = true;
            }
        }

        step
    }

    fn handle_will_wont(&mut self, command: u8, option: u8, output: &mut Vec<u8>) {
        if self.state.willack[option as usize] == 0 {
            self.state.willack[option as usize] = WONT;
        }
        self.state
            .push_command(output, self.state.willack[option as usize], option);

        if command == WILL && option == TTYPE {
            output.extend_from_slice(&[IAC, SB, TTYPE, SEND, IAC, SE]);
        }
    }

    fn handle_do_dont(&mut self, option: u8, output: &mut Vec<u8>) {
        if self.state.options[option as usize] == 0 {
            self.state.options[option as usize] = DONT;
        }
        self.state
            .push_command(output, self.state.options[option as usize], option);
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
            parse_subnegotiation(&[TTYPE, 0, b'x', b't', b'e', b'r', b'm']),
            Some(Subnegotiation::TerminalType("xterm".to_string()))
        );
    }

    #[test]
    fn parses_window_size_subnegotiation() {
        assert_eq!(
            parse_subnegotiation(&[NAWS, 0, 120, 0, 40]),
            Some(Subnegotiation::WindowSize(TerminalSize::new(120, 40)))
        );
    }

    #[test]
    fn ignores_incomplete_or_unknown_subnegotiation() {
        assert_eq!(parse_subnegotiation(&[]), None);
        assert_eq!(parse_subnegotiation(&[TTYPE]), None);
        assert_eq!(parse_subnegotiation(&[NAWS, 0, 80, 0]), None);
        assert_eq!(parse_subnegotiation(&[NEW_ENVIRON, 0]), None);
    }

    #[test]
    fn parser_emits_negotiation_commands() {
        assert_eq!(
            parser_events(&[IAC, WILL, TTYPE]),
            vec![TelnetEvent::Negotiation {
                command: WILL,
                option: TTYPE,
            }]
        );
    }

    #[test]
    fn parser_emits_subnegotiation_payloads() {
        assert_eq!(
            parser_events(&[IAC, SB, NAWS, 0, 80, 0, 24, IAC, SE]),
            vec![TelnetEvent::Subnegotiation(vec![NAWS, 0, 80, 0, 24])]
        );
    }

    #[test]
    fn parser_keeps_subnegotiation_mode_after_embedded_commands() {
        assert_eq!(
            parser_events(&[IAC, SB, b'a', IAC, NOP, b'b', IAC, SE]),
            vec![
                TelnetEvent::Command(NOP),
                TelnetEvent::Subnegotiation(vec![b'a', b'b']),
            ]
        );
    }

    #[test]
    fn initial_output_advertises_supported_options() {
        let mut negotiation = TelnetNegotiation::new();
        let output = negotiation.initial_output();

        assert!(contains(&output, &[IAC, WONT, ECHO]));
        assert!(contains(&output, &[IAC, WILL, SGA]));
        assert!(contains(&output, &[IAC, DO, TTYPE]));
        assert!(contains(&output, &[IAC, DO, NAWS]));
        assert!(contains(&output, &[IAC, DONT, LINEMODE]));
        assert!(contains(&output, &[IAC, WONT, NEW_ENVIRON]));
    }

    #[test]
    fn will_ttype_requests_terminal_type() {
        let mut negotiation = TelnetNegotiation::new();
        let _ = negotiation.initial_output();

        let step = negotiation.handle_event(TelnetEvent::Negotiation {
            command: WILL,
            option: TTYPE,
        });

        assert_eq!(step.output, vec![IAC, SB, TTYPE, SEND, IAC, SE]);
        assert!(!step.extend_deadline);
    }

    #[test]
    fn subnegotiation_updates_telnet_info() {
        let mut negotiation = TelnetNegotiation::new();

        let step = negotiation.handle_event(TelnetEvent::Subnegotiation(vec![
            TTYPE, 0, b'v', b't', b'1', b'0', b'0',
        ]));

        assert!(step.extend_deadline);
        assert_eq!(negotiation.info.term.as_deref(), Some("vt100"));
        assert!(!negotiation.is_complete());

        let step = negotiation.handle_event(TelnetEvent::Subnegotiation(vec![NAWS, 0, 80, 0, 24]));

        assert!(step.extend_deadline);
        assert_eq!(negotiation.info.size, Some(TerminalSize::new(80, 24)));
        assert!(negotiation.is_complete());
    }

    #[test]
    fn end_negotiation_marks_negotiation_complete() {
        let mut negotiation = TelnetNegotiation::new();

        let step = negotiation.handle_event(TelnetEvent::EndNegotiation);

        assert_eq!(step, NegotiationStep::default());
        assert!(negotiation.is_complete());
    }

    #[test]
    fn negotiate_telnet_reads_scripted_terminal_info() {
        let mut input = ScriptedByteSource::new(vec![
            IAC, WILL, TTYPE, IAC, SB, TTYPE, 0, b'x', b't', b'e', b'r', b'm', IAC, SE, IAC, SB,
            NAWS, 0, 100, 0, 40, IAC, SE,
        ]);
        let mut output = Vec::new();

        let info = negotiate_telnet_with_source(&mut output, &mut input).unwrap();

        assert_eq!(info.term.as_deref(), Some("xterm"));
        assert_eq!(info.size, Some(TerminalSize::new(100, 40)));
        assert!(contains(&output, &[IAC, DO, TTYPE]));
        assert!(contains(&output, &[IAC, DO, NAWS]));
        assert!(contains(&output, &[IAC, SB, TTYPE, SEND, IAC, SE]));
    }

    #[test]
    fn negotiate_telnet_stops_when_scripted_input_ends() {
        let mut input = ScriptedByteSource::new(Vec::new());
        let mut output = Vec::new();

        let info = negotiate_telnet_with_source(&mut output, &mut input).unwrap();

        assert_eq!(info, TelnetInfo::default());
        assert!(contains(&output, &[IAC, DO, TTYPE]));
        assert!(contains(&output, &[IAC, DO, NAWS]));
    }
}
