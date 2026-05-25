use crate::animation::frame_count;
use crate::cli::FrameLimit;
use std::thread;
use std::time::{Duration, Instant};

pub(super) struct RenderLoop {
    start: Instant,
    target_delay: Duration,
    frame_index: usize,
    frames_rendered: u32,
}

impl RenderLoop {
    pub(super) fn new(target_delay: Duration) -> Self {
        Self {
            start: Instant::now(),
            target_delay,
            frame_index: 0,
            frames_rendered: 0,
        }
    }

    pub(super) fn frame_index(&self) -> usize {
        self.frame_index
    }

    pub(super) fn elapsed_seconds(&self) -> u64 {
        self.start.elapsed().as_secs()
    }

    pub(super) fn finish_frame(
        &mut self,
        frame_start: Instant,
        frame_limit: Option<FrameLimit>,
    ) -> bool {
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        if frame_limit.is_some_and(|limit| self.frames_rendered == limit.get()) {
            return true;
        }

        self.advance_frame();
        self.sleep_remaining_frame_time(frame_start);
        false
    }

    fn advance_frame(&mut self) {
        self.frame_index += 1;
        if self.frame_index == frame_count() {
            self.frame_index = 0;
        }
    }

    fn sleep_remaining_frame_time(&self, frame_start: Instant) {
        let elapsed = frame_start.elapsed();
        if let Some(sleep_time) = self.target_delay.checked_sub(elapsed) {
            thread::sleep(sleep_time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_loop_advances_frame_indices() {
        let mut render_loop = RenderLoop::new(Duration::ZERO);

        assert_eq!(render_loop.frame_index(), 0);
        assert!(!render_loop.finish_frame(Instant::now(), None));
        assert_eq!(render_loop.frame_index(), 1);
    }

    #[test]
    fn render_loop_wraps_frame_indices() {
        let mut render_loop = RenderLoop::new(Duration::ZERO);

        for _ in 0..frame_count() {
            assert!(!render_loop.finish_frame(Instant::now(), None));
        }

        assert_eq!(render_loop.frame_index(), 0);
    }

    #[test]
    fn render_loop_reports_frame_limit_before_advancing() {
        let mut render_loop = RenderLoop::new(Duration::ZERO);

        assert!(render_loop.finish_frame(Instant::now(), FrameLimit::new(1)));
        assert_eq!(render_loop.frame_index(), 0);
    }
}
