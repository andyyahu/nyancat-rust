use std::io::{self, Write};

pub(super) struct FrameBuffer {
    bytes: Vec<u8>,
}

impl FrameBuffer {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn clear(&mut self) {
        self.bytes.clear();
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[cfg(test)]
    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(super) fn push_byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    pub(super) fn push_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Appends `pattern` repeated `count` times. A run of identical cells thus
    /// costs one fill instead of one `extend_from_slice` (memcpy) per cell; a
    /// uniform pattern (e.g. the two-space block) collapses to a single memset.
    pub(super) fn push_repeated(&mut self, pattern: &[u8], count: usize) {
        if count == 0 || pattern.is_empty() {
            return;
        }

        if pattern.iter().all(|&byte| byte == pattern[0]) {
            self.bytes
                .resize(self.bytes.len() + count * pattern.len(), pattern[0]);
        } else {
            self.bytes.reserve(count * pattern.len());
            for _ in 0..count {
                self.bytes.extend_from_slice(pattern);
            }
        }
    }

    pub(super) fn push_newlines(&mut self, telnet: bool, count: usize) {
        for _ in 0..count {
            if telnet {
                self.push_bytes(b"\r\0\n");
            } else {
                self.push_byte(b'\n');
            }
        }
    }

    pub(super) fn push_frame_prefix(&mut self, clear_screen: bool) {
        if clear_screen {
            self.push_bytes(b"\x1b[H");
        } else {
            self.push_bytes(b"\x1b[u");
        }
    }

    pub(super) fn push_spaces(&mut self, count: i32) {
        let count = count.max(0) as usize;
        self.bytes.resize(self.bytes.len() + count, b' ');
    }
}

impl Write for FrameBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.push_bytes(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_repeated_matches_naive_repetition() {
        let mut fast = FrameBuffer::with_capacity(64);
        let mut naive = FrameBuffer::with_capacity(64);

        for (pattern, count) in [
            (&b"  "[..], 5usize),                  // uniform -> memset fast path
            (&b"\xdb\xdb"[..], 3),                 // uniform CP437 block
            (&b"\xe2\x96\x88\xe2\x96\x88"[..], 4), // multi-byte UTF-8 block
            (&b"  "[..], 0),                       // zero count
            (&b""[..], 7),                         // empty pattern
        ] {
            fast.clear();
            naive.clear();
            fast.push_repeated(pattern, count);
            for _ in 0..count {
                naive.push_bytes(pattern);
            }
            assert_eq!(
                fast.as_bytes(),
                naive.as_bytes(),
                "pattern {pattern:?} x{count}"
            );
        }
    }

    #[test]
    fn frame_buffer_uses_terminal_newlines() {
        let mut buffer = FrameBuffer::with_capacity(16);

        buffer.push_newlines(false, 2);

        assert_eq!(buffer.as_bytes(), b"\n\n");
    }

    #[test]
    fn frame_buffer_uses_telnet_newlines() {
        let mut buffer = FrameBuffer::with_capacity(16);

        buffer.push_newlines(true, 2);

        assert_eq!(buffer.as_bytes(), b"\r\0\n\r\0\n");
    }

    #[test]
    fn frame_buffer_prefix_tracks_clear_screen_mode() {
        let mut buffer = FrameBuffer::with_capacity(8);

        buffer.push_frame_prefix(true);
        assert_eq!(buffer.as_bytes(), b"\x1b[H");

        buffer.clear();
        buffer.push_frame_prefix(false);
        assert_eq!(buffer.as_bytes(), b"\x1b[u");
    }
}
