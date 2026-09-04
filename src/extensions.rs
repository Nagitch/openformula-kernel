//! Example Kitu extension registration.

use core::cmp::Ordering;

use crate::registry::required_scalar;
use crate::{
    coerce_number, Argument, CalcError, CalcResult, CoercionPolicy, EvalContext, FunctionRegistry,
    Value,
};

/// Register dependency-light example functions under the `KITU` namespace.
///
/// This currently adds `KITU.CLAMP(value, minimum, maximum)`.
pub fn register_kitu_examples(registry: &mut FunctionRegistry) -> CalcResult<()> {
    registry.register_extension(
        "KITU",
        "CLAMP",
        "Kitu extension example; not an OpenFormula standard function",
        clamp,
    )
}

fn clamp(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    if arguments.len() != 3 {
        return Err(CalcError::value("KITU.CLAMP expects 3 arguments"));
    }
    let value = coerce_number(required_scalar(&arguments[0], "value")?, policy)?;
    let minimum = coerce_number(required_scalar(&arguments[1], "minimum")?, policy)?;
    let maximum = coerce_number(required_scalar(&arguments[2], "maximum")?, policy)?;
    if minimum.partial_cmp(&maximum) == Some(Ordering::Greater) {
        return Err(CalcError::num("KITU.CLAMP minimum exceeds maximum"));
    }
    let result = if value.partial_cmp(&minimum) == Some(Ordering::Less) {
        minimum
    } else if value.partial_cmp(&maximum) == Some(Ordering::Greater) {
        maximum
    } else {
        value
    };
    Ok(Value::Number(result))
}
