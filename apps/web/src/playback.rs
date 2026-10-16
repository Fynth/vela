//! Pure bounded playback order. Indices identify queue occurrences, not tracks.
pub const MAX_QUEUE: usize = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}
impl Repeat {
    pub fn from_label(value: &str) -> Self {
        match value {
            "all" => Self::All,
            "one" => Self::One,
            _ => Self::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Next,
    Previous,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Play(usize),
    SeekStart { resume: bool },
    Stop,
    None,
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackOrder {
    len: usize,
    current: Option<usize>,
    shuffle: bool,
    repeat: Repeat,
    remaining: Vec<usize>,
    history: Vec<usize>,
    cursor: usize,
    next_up: Vec<usize>,
}

impl PlaybackOrder {
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    pub fn matches(&self, len: usize, current: usize) -> bool {
        self.len == len && self.current == (current < len).then_some(current)
    }

    pub fn reset(&mut self, len: usize, current: usize) {
        self.len = len.min(MAX_QUEUE);
        self.current = (current < self.len).then_some(current);
        self.next_up.clear();
        self.restart_cycle();
    }

    fn restart_cycle(&mut self) {
        self.remaining = (0..self.len)
            .filter(|at| Some(*at) != self.current)
            .collect();
        self.history = self.current.into_iter().collect();
        self.cursor = 0;
    }

    pub fn configure(&mut self, shuffle: bool, repeat: Repeat) {
        if self.shuffle != shuffle {
            self.shuffle = shuffle;
            self.restart_cycle();
        }
        self.repeat = repeat;
    }

    /// Preserve all occurrence references when inserting into the visible queue.
    pub fn insert(&mut self, at: usize, next: bool) -> bool {
        if self.len >= MAX_QUEUE || at > self.len {
            return false;
        }
        self.remap(|value| if value >= at { value + 1 } else { value });
        self.len += 1;
        if self.current.is_none() {
            self.current = Some(0);
            self.restart_cycle();
        } else {
            self.remaining.push(at);
            if next {
                self.next_up.insert(0, at);
            }
        }
        true
    }

    pub fn swap(&mut self, a: usize, b: usize) -> bool {
        if a >= self.len || b >= self.len {
            return false;
        }
        self.remap(|value| {
            if value == a {
                b
            } else if value == b {
                a
            } else {
                value
            }
        });
        true
    }

    pub fn remove(&mut self, at: usize) -> bool {
        if at >= self.len || self.current == Some(at) {
            return false;
        }
        let before = self
            .history
            .iter()
            .take(self.cursor + 1)
            .filter(|&&i| i != at)
            .count();
        self.history.retain(|&i| i != at);
        self.cursor = before.saturating_sub(1);
        self.remaining.retain(|&i| i != at);
        self.next_up.retain(|&i| i != at);
        self.remap(|value| if value > at { value - 1 } else { value });
        self.len -= 1;
        true
    }

    fn remap(&mut self, map: impl Fn(usize) -> usize) {
        self.current = self.current.map(&map);
        for at in self
            .remaining
            .iter_mut()
            .chain(&mut self.history)
            .chain(&mut self.next_up)
        {
            *at = map(*at);
        }
    }

    fn remember(&mut self, at: usize) -> Transition {
        self.current = Some(at);
        self.remaining.retain(|&i| i != at);
        self.history.truncate(self.cursor + 1);
        self.history.push(at);
        if self.history.len() > MAX_QUEUE {
            self.history.remove(0);
        }
        self.cursor = self.history.len() - 1;
        Transition::Play(at)
    }

    pub fn transition(&mut self, command: Command, elapsed: f64, entropy: u32) -> Transition {
        let Some(current) = self.current else {
            return Transition::None;
        };
        if command == Command::Previous {
            if elapsed.is_finite() && elapsed > 3.0 {
                return Transition::SeekStart { resume: false };
            }
            if self.shuffle {
                if self.cursor == 0 {
                    return Transition::SeekStart { resume: false };
                }
                self.cursor -= 1;
                let at = self.history[self.cursor];
                self.current = Some(at);
                return Transition::Play(at);
            }
            return match current.checked_sub(1) {
                Some(at) => self.remember(at),
                None if self.repeat == Repeat::All => self.remember(self.len - 1),
                None => Transition::SeekStart { resume: false },
            };
        }
        if command == Command::Ended && self.repeat == Repeat::One {
            return Transition::SeekStart { resume: true };
        }
        if !self.next_up.is_empty() {
            let at = self.next_up.remove(0);
            return self.remember(at);
        }
        if !self.shuffle {
            return if current + 1 < self.len {
                self.remember(current + 1)
            } else if self.repeat == Repeat::All {
                self.remember(0)
            } else {
                Transition::Stop
            };
        }
        if self.cursor + 1 < self.history.len() {
            self.cursor += 1;
            let at = self.history[self.cursor];
            self.current = Some(at);
            return Transition::Play(at);
        }
        let new_cycle = self.remaining.is_empty();
        if new_cycle {
            if self.repeat != Repeat::All {
                return Transition::Stop;
            }
            self.remaining = (0..self.len).collect();
        }
        // Exclude the previous cycle's last item only from the first draw;
        // leave it in the deck so every following cycle is a full permutation.
        let chosen = if new_cycle && self.len > 1 {
            let candidate = entropy as usize % (self.len - 1);
            if candidate >= current {
                candidate + 1
            } else {
                candidate
            }
        } else {
            entropy as usize % self.remaining.len()
        };
        let at = self.remaining.swap_remove(chosen);
        self.remember(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn order(len: usize, at: usize, shuffle: bool, repeat: Repeat) -> PlaybackOrder {
        let mut model = PlaybackOrder::default();
        model.reset(len, at);
        model.configure(shuffle, repeat);
        model
    }
    fn invariants(model: &PlaybackOrder) {
        assert!(model.len <= MAX_QUEUE);
        assert!(model.history.len() <= MAX_QUEUE);
        for at in model
            .current
            .iter()
            .chain(&model.remaining)
            .chain(&model.history)
            .chain(&model.next_up)
        {
            assert!(*at < model.len, "dangling occurrence {at}");
        }
        assert_eq!(
            model.remaining.iter().collect::<HashSet<_>>().len(),
            model.remaining.len()
        );
        if let Some(at) = model.current {
            assert_eq!(model.history[model.cursor], at);
        }
    }

    #[test]
    fn shuffled_cycles_are_permutations_for_many_lengths_and_seeds() {
        for len in 1..=64 {
            for mut entropy in 0_u32..32 {
                let mut model = order(len, len / 2, true, Repeat::Off);
                let mut heard = HashSet::from([len / 2]);
                for _ in 1..len {
                    entropy = entropy.wrapping_mul(1664525).wrapping_add(1013904223);
                    let Transition::Play(at) = model.transition(Command::Next, 0.0, entropy) else {
                        panic!("cycle ended early")
                    };
                    assert!(heard.insert(at), "repeated an occurrence before exhaustion");
                    invariants(&model);
                }
                assert_eq!(heard.len(), len);
                assert_eq!(
                    model.transition(Command::Ended, 0.0, entropy),
                    Transition::Stop
                );
            }
        }
    }

    #[test]
    fn previous_and_forward_follow_actual_shuffle_history() {
        let mut model = order(4, 0, true, Repeat::Off);
        let a = model.transition(Command::Next, 0.0, 2);
        let b = model.transition(Command::Next, 0.0, 0);
        assert_eq!(
            model.transition(Command::Previous, 4.0, 0),
            Transition::SeekStart { resume: false }
        );
        assert_eq!(model.transition(Command::Previous, 0.0, 99), a);
        assert_eq!(model.transition(Command::Next, 0.0, 99), b);
        assert_eq!(model.transition(Command::Previous, 0.0, 99), a);
        assert_eq!(
            model.transition(Command::Previous, 0.0, 99),
            Transition::Play(0)
        );
        assert_eq!(
            model.transition(Command::Previous, 0.0, 99),
            Transition::SeekStart { resume: false }
        );
        invariants(&model);
    }

    #[test]
    fn explicit_next_is_lifo_even_with_shuffle_and_survives_mode_change() {
        let mut model = order(2, 0, true, Repeat::Off);
        assert!(model.insert(1, true));
        assert!(model.insert(1, true));
        model.configure(false, Repeat::Off);
        model.configure(true, Repeat::Off);
        assert_eq!(
            model.transition(Command::Next, 0.0, 999),
            Transition::Play(1)
        );
        assert_eq!(
            model.transition(Command::Next, 0.0, 999),
            Transition::Play(2)
        );
        assert_eq!(
            model.transition(Command::Next, 0.0, 999),
            Transition::Play(3)
        );
        assert_eq!(model.transition(Command::Next, 0.0, 999), Transition::Stop);
        invariants(&model);
    }

    #[test]
    fn sequential_boundaries_and_repeat_one_distinguish_manual_next() {
        let mut model = order(2, 0, false, Repeat::Off);
        assert_eq!(
            model.transition(Command::Previous, 0.0, 0),
            Transition::SeekStart { resume: false }
        );
        model.configure(false, Repeat::One);
        assert_eq!(
            model.transition(Command::Ended, 0.0, 0),
            Transition::SeekStart { resume: true }
        );
        assert_eq!(model.transition(Command::Next, 0.0, 0), Transition::Play(1));
        assert_eq!(model.transition(Command::Next, 0.0, 0), Transition::Stop);
        model.configure(false, Repeat::All);
        assert_eq!(model.transition(Command::Next, 0.0, 0), Transition::Play(0));
        assert_eq!(
            model.transition(Command::Previous, 0.0, 0),
            Transition::Play(1)
        );
        assert_eq!(
            order(0, 0, true, Repeat::All).transition(Command::Next, 0.0, 0),
            Transition::None
        );
        invariants(&model);
    }

    #[test]
    fn repeat_all_avoids_immediate_cycle_repeats_and_bounds_history() {
        let mut model = order(3, 0, true, Repeat::All);
        let mut previous = 0;
        for entropy in 0..10_000 {
            let Transition::Play(at) = model.transition(Command::Ended, 0.0, entropy) else {
                panic!("repeat stopped")
            };
            assert_ne!(at, previous);
            previous = at;
            invariants(&model);
        }
        assert_eq!(
            order(1, 0, true, Repeat::All).transition(Command::Next, 0.0, 0),
            Transition::Play(0)
        );
    }

    #[test]
    fn every_repeated_cycle_includes_every_occurrence() {
        for len in 2..=16 {
            let mut model = order(len, 0, true, Repeat::All);
            for _ in 1..len {
                model.transition(Command::Next, 0.0, 7);
            }
            for cycle in 0..32 {
                let previous = model.current();
                let mut heard = HashSet::new();
                for step in 0..len {
                    let Transition::Play(at) =
                        model.transition(Command::Next, 0.0, (cycle * len + step) as u32)
                    else {
                        panic!("repeat stopped")
                    };
                    if step == 0 {
                        assert_ne!(Some(at), previous);
                    }
                    assert!(heard.insert(at), "repeat cycle repeated an occurrence");
                }
                assert_eq!(heard.len(), len);
                invariants(&model);
            }
        }
    }

    #[test]
    fn queue_edits_remap_history_priorities_and_current_occurrence() {
        let mut model = order(4, 0, true, Repeat::Off);
        assert_eq!(model.transition(Command::Next, 0.0, 1), Transition::Play(2));
        assert!(model.insert(3, true));
        assert!(model.swap(2, 1));
        assert_eq!(model.current(), Some(1));
        assert!(model.remove(0));
        assert_eq!(model.current(), Some(0));
        assert!(!model.remove(0));
        assert_eq!(model.transition(Command::Next, 0.0, 0), Transition::Play(2));
        assert_eq!(
            model.transition(Command::Previous, 0.0, 0),
            Transition::Play(0)
        );
        assert!(model.remove(2));
        invariants(&model);
        assert_eq!(model.transition(Command::Next, 0.0, 0), Transition::Play(1));
        invariants(&model);
    }

    #[test]
    fn invalid_edits_and_large_legacy_queues_are_bounded() {
        let mut model = order(MAX_QUEUE + 1, 0, true, Repeat::Off);
        assert!(!model.insert(1, true));
        assert!(!model.remove(MAX_QUEUE));
        assert!(!model.swap(0, MAX_QUEUE));
        model.reset(0, 0);
        assert!(model.insert(0, true));
        assert_eq!(model.current(), Some(0));
        invariants(&model);
    }
}
