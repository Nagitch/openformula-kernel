//! Typed calculation semantics shared by Tanu Markdown, TSQ1, and Kitu.
//!
//! This crate implements a bounded OpenFormula 1.4 function-semantics subset.
//! It deliberately does not parse formula source or resolve references.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod context;
mod error;
mod number;
mod registry;
mod value;

/// Dependency-light example extensions used by Kitu integrations.
pub mod extensions;

pub use context::{EvalContext, PureContext};
pub use error::{CalcError, CalcErrorKind, CalcResult};
pub use number::Number;
pub use registry::{
    coerce_logical, coerce_number, CoercionPolicy, FunctionHandler, FunctionMetadata,
    FunctionNamespace, FunctionRegistry,
};
pub use value::{Argument, Value};
