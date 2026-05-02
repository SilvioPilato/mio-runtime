use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use crate::TimerId;

pub struct TimerWheel {
    wheel: Vec<Vec<TimerId>>,
    deleted: HashSet<TimerId>,
    cursor: u64,
    next_id: u64,
    last_instant: Instant,
}

impl TimerWheel {
    pub fn new(capacity: Duration) -> Self {
        let slots = capacity.as_millis();
        debug_assert!(slots <= usize::MAX as u128, "capacity overflows usize");
        TimerWheel {
            wheel: vec![vec![]; slots as usize],
            cursor: 0,
            next_id: 0,
            deleted: HashSet::new(),
            last_instant: Instant::now(),
        }
    }

    pub fn insert(&mut self, delay: Duration) -> TimerId {
        assert!(
            delay.as_millis() < self.wheel.len() as u128,
            "delay {:?} exceeds wheel range of {} ms",
            delay,
            self.wheel.len(),
        );
        let slot = ((self.cursor + delay.as_millis() as u64) % self.wheel.len() as u64) as usize;
        self.next_id += 1;
        let timer = TimerId(self.next_id);
        self.wheel[slot].push(timer);
        timer
    }

    pub fn cancel(&mut self, id: TimerId) {
        self.deleted.insert(id);
    }

    pub fn advance(&mut self, instant: Instant) -> Vec<TimerId> {
        let elapsed = instant
            .duration_since(self.last_instant)
            .as_millis()
            .min(self.wheel.len() as u128) as u64;
        let mut res = vec![];
        for c in self.cursor..(self.cursor + elapsed) {
            let index = c as usize % self.wheel.len();
            for id in self.wheel[index].drain(..) {
                if self.deleted.remove(&id) {
                    continue;
                }
                res.push(id);
            }
        }

        self.cursor += elapsed;
        self.last_instant += Duration::from_millis(elapsed);
        res
    }

    pub fn next_deadline(&self) -> Option<Duration> {
        for offset in 0..self.wheel.len() {
            let index = (self.cursor as usize + offset) % self.wheel.len();
            for id in &self.wheel[index] {
                if self.deleted.contains(id) {
                    continue;
                }
                return Some(Duration::from_millis(offset as u64));
            }
        }
        None
    }
}
