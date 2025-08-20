//! Description of timeouts in the CHERIoT RTOS.

/// Quantity used for measuring time for timeouts.  The unit is scheduler ticks.
pub type Ticks = u32;

/// Structure representing a timeout.  This is intended to allow a single
/// instance to be chained across blocking calls.
///
/// Timeouts *may not be stored in the heap*.  Handling timeout structures that
/// may disappear between sleeping and waking is very complicated and would
/// impact a lot of fast paths.  Instead, most functions that take a timeout
/// will simply fail if the timeout is on the heap.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Timeout {
    /// The time that has elapsed during blocking operations for this timeout
    /// structure.  This should be initialised to 0.  It may exceed the initial
    /// value of `remaining` if a higher-priority thread preempts the blocking
    /// thread.
    elapsed: u32,

    /// The remaining time. This is clamped at 0 on subtraction. A special
    /// value of `UnlimitedTimeout` can be set to represent an unlimited
    /// timeout.
    remaining: u32,
}

impl From<Ticks> for Timeout {
    fn from(remaining: Ticks) -> Self {
        Self { elapsed: 0, remaining }
    }
}
