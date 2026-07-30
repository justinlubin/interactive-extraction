////////////////////////////////////////////////////////////////////////////////
// Early cutoff

use std::time::Duration;

use instant::Instant;

/// The type of reasons that a computation may have been cut off early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarlyCutoff {
    TimerExpired,
}

impl std::fmt::Display for EarlyCutoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EarlyCutoff::TimerExpired => write!(f, "TimerExpired"),
        }
    }
}

impl std::error::Error for EarlyCutoff {}

////////////////////////////////////////////////////////////////////////////////
// Timer

#[derive(Debug)]
enum TimerInner {
    Finite { end: Instant },
    Infinite,
}

/// The type of timers; these can be used to cut off a computation early based
/// on a timeout. These are used cooperatively, and [`Timer::tick`] must be
/// called frequently enough so that there is a chance to interrupt the
/// computation.
#[derive(Debug)]
pub struct Timer(TimerInner);

impl Timer {
    /// A finite-duration timer.
    pub fn finite(duration: Duration) -> Self {
        Timer(TimerInner::Finite {
            end: Instant::now() + duration,
        })
    }

    /// An infinite-duration timer (will never cut off the computation).
    pub fn infinite() -> Self {
        Timer(TimerInner::Infinite)
    }
}

impl pbn::Timer for Timer {
    type EarlyCutoff = EarlyCutoff;

    /// Tick the timer (cooperatively check to see if the computation needs to
    /// stop).
    fn tick(&self) -> Result<(), Self::EarlyCutoff> {
        match self.0 {
            TimerInner::Finite { end } => {
                if Instant::now() > end {
                    Err(EarlyCutoff::TimerExpired)
                } else {
                    Ok(())
                }
            }
            TimerInner::Infinite => Ok(()),
        }
    }
}
