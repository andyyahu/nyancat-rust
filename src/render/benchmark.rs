use std::fmt;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BenchmarkReport {
    frames: u32,
    elapsed: Duration,
    bytes: u64,
    max_frame_bytes: usize,
}

impl BenchmarkReport {
    fn frames_per_second(self) -> f64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds == 0.0 {
            0.0
        } else {
            self.frames as f64 / seconds
        }
    }

    fn average_frame_bytes(self) -> f64 {
        if self.frames == 0 {
            0.0
        } else {
            self.bytes as f64 / self.frames as f64
        }
    }

    fn throughput_mib_per_second(self) -> f64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds == 0.0 {
            0.0
        } else {
            self.bytes as f64 / 1_048_576.0 / seconds
        }
    }
}

impl fmt::Display for BenchmarkReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "benchmark: frames={} elapsed_s={:.6} fps={:.2} bytes={} avg_frame_bytes={:.2} max_frame_bytes={} throughput_mib_s={:.2}",
            self.frames,
            self.elapsed.as_secs_f64(),
            self.frames_per_second(),
            self.bytes,
            self.average_frame_bytes(),
            self.max_frame_bytes,
            self.throughput_mib_per_second()
        )
    }
}

pub(super) struct BenchmarkTracker {
    start: Instant,
    frames: u32,
    bytes: u64,
    max_frame_bytes: usize,
}

impl BenchmarkTracker {
    pub(super) fn new() -> Self {
        Self {
            start: Instant::now(),
            frames: 0,
            bytes: 0,
            max_frame_bytes: 0,
        }
    }

    pub(super) fn record_frame(&mut self, frame_bytes: usize) {
        self.frames = self.frames.saturating_add(1);
        self.bytes = self.bytes.saturating_add(frame_bytes as u64);
        self.max_frame_bytes = self.max_frame_bytes.max(frame_bytes);
    }

    pub(super) fn finish(self) -> BenchmarkReport {
        BenchmarkReport {
            frames: self.frames,
            elapsed: self.start.elapsed(),
            bytes: self.bytes,
            max_frame_bytes: self.max_frame_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_report_calculates_rates() {
        let report = BenchmarkReport {
            frames: 100,
            elapsed: Duration::from_millis(250),
            bytes: 1_048_576,
            max_frame_bytes: 12_345,
        };

        assert_eq!(report.frames_per_second(), 400.0);
        assert_eq!(report.average_frame_bytes(), 10_485.76);
        assert_eq!(report.throughput_mib_per_second(), 4.0);
    }

    #[test]
    fn benchmark_report_formats_stable_key_value_output() {
        let report = BenchmarkReport {
            frames: 2,
            elapsed: Duration::from_secs(1),
            bytes: 100,
            max_frame_bytes: 60,
        };

        assert_eq!(
            report.to_string(),
            "benchmark: frames=2 elapsed_s=1.000000 fps=2.00 bytes=100 avg_frame_bytes=50.00 max_frame_bytes=60 throughput_mib_s=0.00"
        );
    }
}
