use crate::{CalcError, CalcErrorKind, CalcResult, Number};

/// Host-provided capabilities for impure or time-dependent functions.
///
/// The initial standard function subset is pure, but the context makes future
/// clock and random functions injectable rather than ambient.
pub trait EvalContext {
    /// Return the current Unix timestamp in milliseconds.
    fn now_utc_millis(&mut self) -> CalcResult<i64> {
        Err(CalcError::new(
            CalcErrorKind::Unsupported,
            "clock capability is unavailable",
        ))
    }

    /// Return a random number in the half-open interval `[0, 1)`.
    fn random_unit(&mut self) -> CalcResult<Number> {
        Err(CalcError::new(
            CalcErrorKind::Unsupported,
            "random capability is unavailable",
        ))
    }
}

/// A context that rejects every impure capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct PureContext;

impl EvalContext for PureContext {}
