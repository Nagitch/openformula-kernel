use alloc::string::String;
use alloc::vec::Vec;

use crate::{CalcError, Number};

/// A scalar value understood by the calculation kernel.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A finite calculation number.
    Number(Number),
    /// UTF-8 text.
    Text(String),
    /// A distinct logical value.
    Boolean(bool),
    /// An empty cell or absent scalar value.
    Empty,
    /// A typed calculation error carried as a value for propagation.
    Error(CalcError),
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Number(Number::from_i64(value))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

impl From<CalcError> for Value {
    fn from(value: CalcError) -> Self {
        Self::Error(value)
    }
}

/// An evaluated function argument retaining whether values came from a range.
#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    /// One directly supplied scalar.
    Scalar(Value),
    /// Values supplied through a host-resolved range or reference list.
    Range(Vec<Value>),
    /// An explicitly omitted optional parameter.
    Missing,
}

impl Argument {
    /// Construct a scalar argument.
    #[must_use]
    pub fn scalar(value: impl Into<Value>) -> Self {
        Self::Scalar(value.into())
    }

    /// Construct a range argument in deterministic source order.
    #[must_use]
    pub fn range(values: impl IntoIterator<Item = Value>) -> Self {
        Self::Range(values.into_iter().collect())
    }
}

impl From<Value> for Argument {
    fn from(value: Value) -> Self {
        Self::Scalar(value)
    }
}
