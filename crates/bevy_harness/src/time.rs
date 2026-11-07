//! Seconds as a test writes them, and ticks as an app counts them.

use core::time::Duration;

/// How long a tick is unless the builder or the app says: 64 a second.
pub(crate) const STEP: Duration = Duration::from_micros(15_625);

/// `seconds` as a [`Duration`].
///
/// # Panics
///
/// If `seconds` is negative or not finite: a test asking for that is
/// wrong, and waiting no time instead would let it pass.
#[track_caller]
pub(crate) fn seconds(seconds: f32) -> Duration {
    Duration::try_from_secs_f32(seconds)
        .unwrap_or_else(|_| panic!("{seconds} isn't a length of time: it has to be 0 or more"))
}

/// How long a tick is at `hz` ticks a second.
///
/// # Panics
///
/// If `hz` isn't more than 0 and finite, or so far from 1 that a tick is
/// no time or more than a `Duration` holds.
#[track_caller]
pub(crate) fn step(hz: f64) -> Duration {
    Some(hz)
        .filter(|hz| hz.is_finite() && *hz > 0.0)
        .and_then(|hz| Duration::try_from_secs_f64(1.0 / hz).ok())
        .filter(|step| !step.is_zero())
        .unwrap_or_else(|| panic!("{hz} isn't a tick rate: it has to be more than 0, and finite"))
}

/// How many ticks of `step` `length` is, rounded to the nearest.
pub(crate) fn nearest(length: Duration, step: Duration) -> u32 {
    let step = step.as_nanos().max(1);
    u32::try_from((length.as_nanos() + step / 2) / step).unwrap_or(u32::MAX)
}

/// How many ticks of `step` `length` is, rounded up: enough to cover it.
pub(crate) fn covering(length: Duration, step: Duration) -> u32 {
    let step = step.as_nanos().max(1);
    u32::try_from(length.as_nanos().div_ceil(step)).unwrap_or(u32::MAX)
}

/// Ticks a second, at `step` a tick, to a thousandth: for a failure.
pub(crate) fn hz(step: Duration) -> f64 {
    (1000.0 / step.as_secs_f64()).round() / 1000.0
}
