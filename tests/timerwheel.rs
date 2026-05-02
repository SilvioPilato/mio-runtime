use std::time::{Duration, Instant};

use mio_runtime::{TimerId, TimerWheel};

const SLOTS: usize = 512;

// --- TimerId allocation ---

#[test]
fn insert_returns_monotonically_increasing_ids() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let a = wheel.insert(Duration::from_millis(0));
    let b = wheel.insert(Duration::from_millis(0));
    let c = wheel.insert(Duration::from_millis(0));
    assert!(a.0 < b.0);
    assert!(b.0 < c.0);
}

// --- next_deadline: spec's three positions ---

#[test]
fn next_deadline_in_current_slot() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    wheel.insert(Duration::from_millis(0));
    assert_eq!(wheel.next_deadline(), Some(Duration::from_millis(0)));
}

#[test]
fn next_deadline_in_next_slot() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    wheel.insert(Duration::from_millis(1));
    assert_eq!(wheel.next_deadline(), Some(Duration::from_millis(1)));
}

#[test]
fn next_deadline_in_slot_just_before_wraparound() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let max = (SLOTS - 1) as u64;
    wheel.insert(Duration::from_millis(max));
    assert_eq!(wheel.next_deadline(), Some(Duration::from_millis(max)));
}

// --- next_deadline: edge cases ---

#[test]
fn next_deadline_returns_none_on_empty_wheel() {
    let wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    assert_eq!(wheel.next_deadline(), None);
}

#[test]
fn next_deadline_returns_none_when_only_cancelled_timers_exist() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let id = wheel.insert(Duration::from_millis(5));
    wheel.cancel(id);
    assert_eq!(wheel.next_deadline(), None);
}

#[test]
fn next_deadline_skips_cancelled_returns_first_live() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let cancelled = wheel.insert(Duration::from_millis(2));
    wheel.insert(Duration::from_millis(7));
    wheel.cancel(cancelled);
    assert_eq!(wheel.next_deadline(), Some(Duration::from_millis(7)));
}

// --- advance: lazy cancellation ---

#[test]
fn advance_does_not_return_cancelled_timer() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let id = wheel.insert(Duration::from_millis(2));
    wheel.cancel(id);
    let fired = wheel.advance(t + Duration::from_millis(10));
    assert!(fired.is_empty());
}

#[test]
fn advance_returns_only_non_cancelled_when_mixed() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let cancelled = wheel.insert(Duration::from_millis(1));
    let live = wheel.insert(Duration::from_millis(2));
    wheel.cancel(cancelled);
    let fired = wheel.advance(t + Duration::from_millis(10));
    assert_eq!(fired, vec![live]);
}

#[test]
fn cancel_of_unknown_id_does_not_affect_live_timers() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let live = wheel.insert(Duration::from_millis(1));
    wheel.cancel(TimerId(9_999_999));
    let fired = wheel.advance(t + Duration::from_millis(10));
    assert_eq!(fired, vec![live]);
}

// --- advance: chronological order ---

#[test]
fn advance_returns_ids_in_chronological_order() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let third = wheel.insert(Duration::from_millis(5));
    let first = wheel.insert(Duration::from_millis(1));
    let second = wheel.insert(Duration::from_millis(3));
    let fired = wheel.advance(t + Duration::from_millis(10));
    assert_eq!(fired, vec![first, second, third]);
}

#[test]
fn advance_returns_same_slot_in_insertion_order() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let a = wheel.insert(Duration::from_millis(2));
    let b = wheel.insert(Duration::from_millis(2));
    let c = wheel.insert(Duration::from_millis(2));
    let fired = wheel.advance(t + Duration::from_millis(10));
    assert_eq!(fired, vec![a, b, c]);
}

// --- advance: slot boundaries ---

#[test]
fn advance_does_not_fire_timer_before_its_slot() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    wheel.insert(Duration::from_millis(5));
    let fired = wheel.advance(t + Duration::from_millis(4));
    assert!(fired.is_empty());
}

#[test]
fn advance_fires_timer_at_its_slot() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let id = wheel.insert(Duration::from_millis(5));
    let fired = wheel.advance(t + Duration::from_millis(6));
    assert_eq!(fired, vec![id]);
}

// --- advance: cursor wraparound ---

#[test]
fn advance_fires_timer_scheduled_across_wraparound() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    // Move cursor close to the end of the wheel.
    let near_end = (SLOTS - 5) as u64;
    wheel.advance(t + Duration::from_millis(near_end));
    // Schedule a timer 8 ms further: lands on slot 3 after wraparound.
    let id = wheel.insert(Duration::from_millis(8));
    let fired = wheel.advance(t + Duration::from_millis(near_end + 9));
    assert_eq!(fired, vec![id]);
}

#[test]
fn next_deadline_after_advance_reflects_new_cursor() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    wheel.insert(Duration::from_millis(10));
    wheel.advance(t + Duration::from_millis(4));
    // 10 ms originally - 4 ms elapsed = 6 ms remaining.
    assert_eq!(wheel.next_deadline(), Some(Duration::from_millis(6)));
}

// --- advance: idempotence and clamping ---

#[test]
fn advance_with_no_elapsed_time_returns_empty() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    wheel.insert(Duration::from_millis(5));
    let fired = wheel.advance(t);
    assert!(fired.is_empty());
}

#[test]
fn multi_revolution_advance_does_not_double_fire() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    let t = Instant::now();
    let id = wheel.insert(Duration::from_millis(50));
    let fired = wheel.advance(t + Duration::from_millis((SLOTS * 2) as u64));
    assert_eq!(fired, vec![id]);
}

// --- insert: range assertion ---

#[test]
#[should_panic(expected = "exceeds wheel range")]
fn insert_panics_when_delay_equals_wheel_range() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    wheel.insert(Duration::from_millis(SLOTS as u64));
}

#[test]
fn insert_at_max_supported_delay_does_not_panic() {
    let mut wheel = TimerWheel::new(Duration::from_millis(SLOTS as u64));
    wheel.insert(Duration::from_millis((SLOTS - 1) as u64));
}
