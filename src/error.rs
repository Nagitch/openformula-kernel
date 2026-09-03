use alloc::string::{String, ToString};
use core::fmt;

/// Result returned by calculation-kernel operations.
pub type CalcResult<T = crate::Value> = Result<T, CalcError>;

/// Stable categories for calculation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalcErrorKind {
    /// An empty range intersection or equivalent null reference.
    Null,
    /// Division by zero or an equivalent zero denominator.
    DivZero,
    /// A value has the wrong type or shape.
    Value,
    /// A reference supplied by a host is invalid.
    Ref,
    /// A function or identifier is unknown.
    Name,
    /// A numeric domain, range, or finite-result requirement failed.
    Num,
    /// A requested value is unavailable.
    NotAvailable,
    /// A configured evaluation bound was exceeded.
    Limit,
    /// The host did not provide a required impure capability.
    Unsupported,
}

impl CalcErrorKind {
    /// Return the spreadsheet-style stable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Null => "#NULL!",
            Self::DivZero => "#DIV/0!",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Name => "#NAME?",
            Self::Num => "#NUM!",
            Self::NotAvailable => "#N/A",
            Self::Limit => "#LIMIT!",
            Self::Unsupported => "#UNSUPPORTED!",
        }
    }
}

/// A typed calculation error without parser- or storage-specific location data.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalcError {
    kind: CalcErrorKind,
    message: String,
}

impl CalcError {
    /// Construct an error in a stable category.
    #[must_use]
    pub fn new(kind: CalcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Return the stable error category.
    #[must_use]
    pub const fn kind(&self) -> CalcErrorKind {
        self.kind
    }

    /// Return the spreadsheet-style error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Return the human-readable diagnostic detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn value(message: impl Into<String>) -> Self {
        Self::new(CalcErrorKind::Value, message)
    }

    pub(crate) fn num(message: impl Into<String>) -> Self {
        Self::new(CalcErrorKind::Num, message)
    }

    pub(crate) fn name(message: impl Into<String>) -> Self {
        Self::new(CalcErrorKind::Name, message)
    }
}

impl fmt::Display for CalcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.code(), self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CalcError {}

impl From<CalcErrorKind> for CalcError {
    fn from(kind: CalcErrorKind) -> Self {
        Self::new(kind, kind.code().to_string())
    }
}
