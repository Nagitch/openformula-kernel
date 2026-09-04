use core::cmp::Ordering;
use core::fmt;

use crate::{CalcError, CalcResult};

#[derive(Clone, Copy, Debug)]
enum Repr {
    Integer(i64),
    Real(f64),
}

/// A finite calculation number with a deliberately private representation.
#[derive(Clone, Copy, Debug)]
pub struct Number(Repr);

impl Number {
    /// Construct an exactly represented signed integer.
    #[must_use]
    pub const fn from_i64(value: i64) -> Self {
        Self(Repr::Integer(value))
    }

    /// Construct a number from a finite binary64 value.
    ///
    /// Negative zero is normalized to positive zero. NaN and infinities return
    /// `#NUM!`.
    pub fn try_from_f64(value: f64) -> CalcResult<Self> {
        if !value.is_finite() {
            return Err(CalcError::num("number must be finite"));
        }
        Ok(Self(Repr::Real(normalize_zero(value))))
    }

    /// Return the number as binary64 for host interoperation.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        match self.0 {
            Repr::Integer(value) => value as f64,
            Repr::Real(value) => value,
        }
    }

    /// Return the original integer representation when it is preserved.
    #[must_use]
    pub const fn as_i64(self) -> Option<i64> {
        match self.0 {
            Repr::Integer(value) => Some(value),
            Repr::Real(_) => None,
        }
    }

    /// Return an integer when the mathematical value is exactly representable
    /// as `i64`, regardless of its internal representation.
    #[must_use]
    pub fn as_i64_exact(self) -> Option<i64> {
        match self.0 {
            Repr::Integer(value) => Some(value),
            Repr::Real(value)
                if value >= i64::MIN as f64
                    && value < -(i64::MIN as f64)
                    && libm::trunc(value) == value =>
            {
                Some(value as i64)
            }
            Repr::Real(_) => None,
        }
    }

    /// Return whether this number is zero.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.as_f64() == 0.0
    }

    /// Return whether this number is mathematically integral.
    #[must_use]
    pub fn is_integral(self) -> bool {
        match self.0 {
            Repr::Integer(_) => true,
            Repr::Real(value) => libm::trunc(value) == value,
        }
    }

    pub(crate) fn checked_add(self, other: Self) -> CalcResult<Self> {
        match (self.0, other.0) {
            (Repr::Integer(left), Repr::Integer(right)) => left
                .checked_add(right)
                .map(Self::from_i64)
                .ok_or_else(|| CalcError::num("integer addition overflow")),
            _ => Self::try_from_f64(self.as_f64() + other.as_f64()),
        }
    }

    pub(crate) fn checked_abs(self) -> CalcResult<Self> {
        match self.0 {
            Repr::Integer(value) => value
                .checked_abs()
                .map(Self::from_i64)
                .ok_or_else(|| CalcError::num("integer absolute-value overflow")),
            Repr::Real(value) => Self::try_from_f64(libm::fabs(value)),
        }
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::from_i64(value)
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.as_f64() == other.as_f64()
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_f64().partial_cmp(&other.as_f64())
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Repr::Integer(value) => value.fmt(formatter),
            Repr::Real(value) => value.fmt(formatter),
        }
    }
}

pub(crate) fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}
