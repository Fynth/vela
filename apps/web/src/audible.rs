//! Conservative elapsed-listening policy. Media fetches and seek distances are not listens.
use serde::{Deserialize, Serialize};
use vela_domain::ListeningProgress;

pub const MAX_PENDING: usize = 100;
pub const MAX_OUTBOX_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub wall_ms: f64,
    pub position: f64,
    pub rate: f64,
    pub audible: bool,
}

#[derive(Default)]
pub struct AudibleClock {
    previous: Option<Sample>,
    fraction_ms: f64,
}
impl AudibleClock {
    pub fn reset(&mut self) {
        self.previous = None;
        self.fraction_ms = 0.0;
    }
    /// Attribute the interval to its previous state, including the final pause/EOF residual.
    /// A discontinuity discards that interval; no background timer invents elapsed audio.
    pub fn observe(&mut self, sample: Sample) -> i64 {
        if !sample.wall_ms.is_finite()
            || !sample.position.is_finite()
            || sample.position < 0.0
            || !sample.rate.is_finite()
            || !(0.25..=4.0).contains(&sample.rate)
        {
            self.reset();
            return 0;
        }
        let previous = self.previous.replace(sample);
        let Some(previous) = previous.filter(|value| value.audible) else {
            return 0;
        };
        let wall = sample.wall_ms - previous.wall_ms;
        let media = (sample.position - previous.position) * 1000.0;
        if !(0.0..=2000.0).contains(&wall)
            || wall == 0.0
            || media <= 0.0
            || media > wall * previous.rate + 150.0
            || (sample.rate - previous.rate).abs() > f64::EPSILON
        {
            return 0;
        }
        self.fraction_ms += (media / previous.rate).min(wall);
        let whole = self.fraction_ms.floor() as i64;
        self.fraction_ms -= whole as f64;
        whole
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outbox {
    pub version: u8,
    pub owner: String,
    pub device: String,
    pub revision: i64,
    pub events: Vec<ListeningProgress>,
}
pub fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
impl Outbox {
    pub fn decode(raw: &str, owner: &str, device: &str, revision: i64) -> Option<Self> {
        if raw.len() > MAX_OUTBOX_BYTES {
            return None;
        }
        let value: Self = serde_json::from_str(raw).ok()?;
        (value.version == 1
            && value.owner == owner
            && value.device == device
            && value.revision == revision
            && canonical_uuid(owner)
            && canonical_uuid(device)
            && value.events.len() <= MAX_PENDING
            && value.events.iter().all(|event| {
                canonical_uuid(&event.session_id)
                    && canonical_uuid(&event.track_id)
                    && (1..=i32::MAX as u32).contains(&event.sequence)
                    && (0..=86_400_000).contains(&event.listened_ms)
                    && (0..=604_800_000).contains(&event.position_ms)
                    && event.started_at >= 0
            }))
        .then_some(value)
    }
}

/// At most one latest cumulative observation per occurrence, never an unbounded event log.
pub fn enqueue(events: &mut Vec<ListeningProgress>, event: ListeningProgress) {
    if let Some(existing) = events
        .iter_mut()
        .find(|old| old.session_id == event.session_id)
    {
        if event.sequence > existing.sequence {
            *existing = event;
        }
        return;
    }
    if events.len() >= MAX_PENDING {
        events.remove(0);
    }
    events.push(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(wall_ms: f64, position: f64, audible: bool) -> Sample {
        Sample {
            wall_ms,
            position,
            rate: 1.0,
            audible,
        }
    }
    #[test]
    fn advancing_audio_and_pause_residual_only() {
        let mut clock = AudibleClock::default();
        assert_eq!(clock.observe(sample(0.0, 0.0, true)), 0);
        assert_eq!(clock.observe(sample(500.0, 0.5, true)), 500);
        assert_eq!(clock.observe(sample(750.0, 0.75, false)), 250);
        assert_eq!(clock.observe(sample(1000.0, 0.75, false)), 0);
        assert_eq!(clock.observe(sample(1250.0, 0.75, true)), 0);
        assert_eq!(clock.observe(sample(1500.0, 1.0, true)), 250);
    }
    #[test]
    fn rejects_seeks_clock_reversals_long_gaps_and_nonfinite_values() {
        let mut clock = AudibleClock::default();
        for value in [
            sample(0.0, 0.0, true),
            sample(250.0, 20.0, true),
            sample(500.0, 2.0, true),
            sample(5000.0, 6.5, true),
            sample(4990.0, 7.0, true),
            sample(f64::NAN, 7.1, true),
        ] {
            assert_eq!(clock.observe(value), 0);
        }
        assert_eq!(clock.observe(sample(5250.0, 7.2, true)), 0);
        assert_eq!(clock.observe(sample(5500.0, 7.45, true)), 250);
    }
    #[test]
    fn inaudible_state_does_not_count_and_playback_rate_counts_wall_time() {
        let mut clock = AudibleClock::default();
        assert_eq!(clock.observe(sample(0.0, 0.0, false)), 0);
        assert_eq!(clock.observe(sample(500.0, 0.5, false)), 0);
        clock.reset();
        let first = Sample {
            wall_ms: 0.0,
            position: 0.0,
            rate: 2.0,
            audible: true,
        };
        assert_eq!(clock.observe(first), 0);
        assert_eq!(
            clock.observe(Sample {
                wall_ms: 500.0,
                position: 1.0,
                ..first
            }),
            500
        );
    }
    #[test]
    fn fractional_observations_retain_milliseconds() {
        let mut clock = AudibleClock::default();
        let mut total = 0;
        for at in 0..=100 {
            total += clock.observe(sample(at as f64 * 0.5, at as f64 * 0.0005, true));
        }
        assert!((49..=50).contains(&total));
    }
    #[test]
    fn bounded_outbox_merges_sequences_and_checks_owner_revision() {
        let owner = uuid::Uuid::new_v4().to_string();
        let device = uuid::Uuid::new_v4().to_string();
        let mut events = Vec::new();
        for _ in 0..105 {
            enqueue(
                &mut events,
                ListeningProgress {
                    session_id: uuid::Uuid::new_v4().to_string(),
                    sequence: 1,
                    track_id: uuid::Uuid::new_v4().to_string(),
                    listened_ms: 1000,
                    position_ms: 1000,
                    started_at: 1,
                    ended: false,
                },
            );
        }
        assert_eq!(events.len(), MAX_PENDING);
        let mut event = events[0].clone();
        event.sequence = 2;
        event.listened_ms = 2000;
        enqueue(&mut events, event.clone());
        assert_eq!(events[0], event);
        event.sequence = 1;
        enqueue(&mut events, event);
        assert_eq!(events[0].sequence, 2);
        let raw = serde_json::to_string(&Outbox {
            version: 1,
            owner: owner.clone(),
            device: device.clone(),
            revision: 7,
            events,
        })
        .unwrap();
        assert!(Outbox::decode(&raw, &owner, &device, 7).is_some());
        assert!(Outbox::decode(&raw, &owner, &device, 8).is_none());
        assert!(Outbox::decode(&raw, &uuid::Uuid::new_v4().to_string(), &device, 7).is_none());
        assert!(Outbox::decode(&" ".repeat(MAX_OUTBOX_BYTES + 1), &owner, &device, 7).is_none());
    }
}
