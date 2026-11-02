//! Small, revision-bound scope telemetry; never owns a camera frame.
use std::time::{Duration, Instant};

pub struct Histogram {
    revision: u64,
    collected: Instant,
    bins: [u32; 96],
}
impl Histogram {
    pub fn new(revision: u64, collected: Instant, bins: [u32; 96]) -> Self {
        Self {
            revision,
            collected,
            bins,
        }
    }
    pub fn current(&self, revision: Option<u64>, now: Instant) -> Option<&[u32; 96]> {
        (revision == Some(self.revision)
            && now.checked_duration_since(self.collected)? < Duration::from_millis(500))
        .then_some(&self.bins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_revision_expired_or_future_scope_never_replaces_source() {
        let now = Instant::now();
        let histogram = Histogram::new(7, now, [96; 96]);
        assert_eq!(histogram.current(Some(7), now), Some(&[96; 96]));
        assert!(histogram.current(Some(8), now).is_none());
        assert!(histogram.current(None, now).is_none());
        assert!(
            histogram
                .current(Some(7), now + Duration::from_millis(499))
                .is_some()
        );
        assert!(
            histogram
                .current(Some(7), now + Duration::from_millis(500))
                .is_none()
        );
        assert!(
            histogram
                .current(Some(7), now - Duration::from_millis(1))
                .is_none()
        );
    }
}
