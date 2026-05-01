use crate::sys;
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
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    return Ok(None);
                }
                return Err(err);
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

pub(crate) struct TelnetInfo {
    pub(crate) term: Option<String>,
    pub(crate) width: Option<i32>,
    pub(crate) height: Option<i32>,
}

pub(crate) fn negotiate_telnet(out: &mut impl Write) -> io::Result<TelnetInfo> {
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
