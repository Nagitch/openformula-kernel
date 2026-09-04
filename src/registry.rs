use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::number::normalize_zero;
use crate::{Argument, CalcError, CalcErrorKind, CalcResult, EvalContext, Number, Value};

/// Controls host-selected implicit conversions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoercionPolicy {
    /// OpenFormula conversions with locale-neutral text parsing.
    #[default]
    OpenFormula,
    /// Accept only exact logical and numeric types; aggregates still ignore
    /// empty values.
    Strict,
}

/// Whether a registered function is standard or explicitly namespaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionNamespace {
    /// An OpenFormula-derived standard function.
    Standard,
    /// A host extension namespace such as `KITU`.
    Extension(String),
}

/// Compatibility information attached to every registry entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMetadata {
    name: String,
    namespace: FunctionNamespace,
    openformula_reference: Option<&'static str>,
    notes: &'static str,
}

impl FunctionMetadata {
    /// Return the case-normalized callable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the standard or extension namespace.
    #[must_use]
    pub const fn namespace(&self) -> &FunctionNamespace {
        &self.namespace
    }

    /// Return the normative OpenFormula 1.4 section, when applicable.
    #[must_use]
    pub const fn openformula_reference(&self) -> Option<&'static str> {
        self.openformula_reference
    }

    /// Return concise compatibility notes.
    #[must_use]
    pub const fn notes(&self) -> &'static str {
        self.notes
    }
}

/// Function pointer accepted by the registry for standard and extension calls.
pub type FunctionHandler = fn(&[Argument], &mut dyn EvalContext, CoercionPolicy) -> CalcResult;

#[derive(Clone)]
struct RegisteredFunction {
    metadata: FunctionMetadata,
    handler: FunctionHandler,
}

/// Case-insensitive registry with separate standard and extension namespaces.
#[derive(Clone)]
pub struct FunctionRegistry {
    functions: BTreeMap<String, RegisteredFunction>,
    coercion: CoercionPolicy,
}

impl FunctionRegistry {
    /// Construct a registry containing the supported standard function subset.
    #[must_use]
    pub fn standard() -> Self {
        Self::standard_with_policy(CoercionPolicy::OpenFormula)
    }

    /// Construct the standard registry with an explicit host coercion policy.
    #[must_use]
    pub fn standard_with_policy(coercion: CoercionPolicy) -> Self {
        let mut registry = Self {
            functions: BTreeMap::new(),
            coercion,
        };
        registry.register_standard(
            "SUM",
            "6.16.61",
            "NumberSequenceList semantics; scalar text conversion is locale-neutral",
            sum,
        );
        registry.register_standard(
            "MIN",
            "6.18.48",
            "returns zero when no numeric value is present",
            min,
        );
        registry.register_standard(
            "MAX",
            "6.18.45",
            "returns zero when no numeric value is present",
            max,
        );
        registry.register_standard(
            "AVERAGE",
            "6.18.3",
            "returns #DIV/0! when no numeric value is present",
            average,
        );
        registry.register_standard(
            "IF",
            "6.15.4",
            "use evaluate_lazy to guarantee unselected-branch short circuiting",
            if_prepared,
        );
        registry.register_standard("ABS", "6.16.2", "integer overflow returns #NUM!", abs);
        registry.register_standard(
            "ROUND",
            "6.17.5",
            "Digits is intentionally bounded to -15..=15",
            round,
        );
        registry.register_standard(
            "FLOOR",
            "6.17.3",
            "two-ULP correction applies only at exact-multiple boundaries",
            floor,
        );
        registry.register_standard(
            "CEILING",
            "6.17.1",
            "two-ULP correction applies only at exact-multiple boundaries",
            ceiling,
        );
        registry.register_standard(
            "POWER",
            "6.16.46",
            "0^0 is 1; negative fractional powers return #NUM!",
            power,
        );
        registry
    }

    /// Return the active coercion policy.
    #[must_use]
    pub const fn coercion_policy(&self) -> CoercionPolicy {
        self.coercion
    }

    /// Register an explicitly namespaced extension function.
    ///
    /// Namespace and local name are ASCII identifiers and are normalized to
    /// uppercase. Standard names cannot be replaced by this API.
    pub fn register_extension(
        &mut self,
        namespace: &str,
        name: &str,
        notes: &'static str,
        handler: FunctionHandler,
    ) -> CalcResult<()> {
        if !valid_identifier(namespace) || !valid_identifier(name) {
            return Err(CalcError::name(
                "extension namespace and name must be ASCII identifiers",
            ));
        }
        let namespace = namespace.to_ascii_uppercase();
        if namespace == "OF" || namespace == "OPENFORMULA" || namespace == "STANDARD" {
            return Err(CalcError::name("extension namespace is reserved"));
        }
        let full_name = format!("{namespace}.{}", name.to_ascii_uppercase());
        if self.functions.contains_key(&full_name) {
            return Err(CalcError::name(format!(
                "function `{full_name}` is already registered"
            )));
        }
        let metadata = FunctionMetadata {
            name: full_name.clone(),
            namespace: FunctionNamespace::Extension(namespace),
            openformula_reference: None,
            notes,
        };
        self.functions
            .insert(full_name, RegisteredFunction { metadata, handler });
        Ok(())
    }

    /// Return metadata for a registered function.
    #[must_use]
    pub fn metadata(&self, name: &str) -> Option<&FunctionMetadata> {
        self.functions
            .get(&name.to_ascii_uppercase())
            .map(|function| &function.metadata)
    }

    /// Iterate over all compatibility metadata in stable name order.
    pub fn all_metadata(&self) -> impl Iterator<Item = &FunctionMetadata> {
        self.functions.values().map(|function| &function.metadata)
    }

    /// Evaluate already-prepared arguments.
    ///
    /// For `IF`, this method selects only the requested prepared branch, but a
    /// caller that needs to avoid evaluating branch expressions must use
    /// [`Self::evaluate_lazy`].
    pub fn evaluate(
        &self,
        name: &str,
        arguments: &[Argument],
        context: &mut dyn EvalContext,
    ) -> CalcResult {
        let normalized = name.to_ascii_uppercase();
        let function = self.functions.get(&normalized).ok_or_else(|| {
            CalcError::new(CalcErrorKind::Name, format!("unknown function `{name}`"))
        })?;
        if normalized == "IF" {
            return if_prepared(arguments, context, self.coercion);
        }
        propagate_leftmost_error(arguments)?;
        (function.handler)(arguments, context, self.coercion)
    }

    /// Evaluate arguments on demand, preserving OpenFormula `IF` short circuiting.
    pub fn evaluate_lazy<F>(
        &self,
        name: &str,
        argument_count: usize,
        mut evaluate_argument: F,
        context: &mut dyn EvalContext,
    ) -> CalcResult
    where
        F: FnMut(usize) -> CalcResult<Argument>,
    {
        let normalized = name.to_ascii_uppercase();
        if !self.functions.contains_key(&normalized) {
            return Err(CalcError::new(
                CalcErrorKind::Name,
                format!("unknown function `{name}`"),
            ));
        }
        if normalized == "IF" {
            if !(1..=3).contains(&argument_count) {
                return Err(CalcError::value("IF expects one to three arguments"));
            }
            let condition = evaluate_argument(0)?;
            let condition =
                coerce_logical(required_scalar(&condition, "IF condition")?, self.coercion)?;
            if argument_count == 1 {
                return Ok(Value::Boolean(condition));
            }
            let selected = if condition { 1 } else { 2 };
            if selected >= argument_count {
                return Ok(Value::Boolean(false));
            }
            let value = evaluate_argument(selected)?;
            return selected_if_value(value);
        }
        let mut arguments = Vec::with_capacity(argument_count);
        for index in 0..argument_count {
            arguments.push(evaluate_argument(index)?);
        }
        self.evaluate(&normalized, &arguments, context)
    }

    fn register_standard(
        &mut self,
        name: &'static str,
        reference: &'static str,
        notes: &'static str,
        handler: FunctionHandler,
    ) {
        let metadata = FunctionMetadata {
            name: name.to_string(),
            namespace: FunctionNamespace::Standard,
            openformula_reference: Some(reference),
            notes,
        };
        self.functions
            .insert(name.to_string(), RegisteredFunction { metadata, handler });
    }
}

/// Convert a scalar to a number under the selected host policy.
pub fn coerce_number(value: &Value, policy: CoercionPolicy) -> CalcResult<Number> {
    match value {
        Value::Number(value) => Ok(*value),
        Value::Error(error) => Err(error.clone()),
        Value::Boolean(value) if policy == CoercionPolicy::OpenFormula => {
            Ok(Number::from_i64(i64::from(*value)))
        }
        Value::Empty if policy == CoercionPolicy::OpenFormula => Ok(Number::from_i64(0)),
        Value::Text(value) if policy == CoercionPolicy::OpenFormula => value
            .trim()
            .parse::<f64>()
            .map_err(|_| CalcError::value("text cannot be converted to a number"))
            .and_then(Number::try_from_f64),
        Value::Boolean(_) | Value::Empty | Value::Text(_) => {
            Err(CalcError::value("expected a number"))
        }
    }
}

/// Convert a scalar to a logical under the selected host policy.
pub fn coerce_logical(value: &Value, policy: CoercionPolicy) -> CalcResult<bool> {
    match value {
        Value::Boolean(value) => Ok(*value),
        Value::Error(error) => Err(error.clone()),
        Value::Number(value) if policy == CoercionPolicy::OpenFormula => Ok(!value.is_zero()),
        Value::Empty if policy == CoercionPolicy::OpenFormula => Ok(false),
        Value::Text(value) if policy == CoercionPolicy::OpenFormula => {
            if value.eq_ignore_ascii_case("TRUE") {
                Ok(true)
            } else if value.eq_ignore_ascii_case("FALSE") {
                Ok(false)
            } else {
                Err(CalcError::value("text cannot be converted to a logical"))
            }
        }
        Value::Number(_) | Value::Empty | Value::Text(_) => {
            Err(CalcError::value("expected a logical"))
        }
    }
}

pub(crate) fn required_scalar<'a>(argument: &'a Argument, label: &str) -> CalcResult<&'a Value> {
    match argument {
        Argument::Scalar(value) => Ok(value),
        Argument::Range(_) => Err(CalcError::value(format!("{label} must be a scalar"))),
        Argument::Missing => Err(CalcError::value(format!("{label} is required"))),
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn propagate_leftmost_error(arguments: &[Argument]) -> CalcResult<()> {
    for argument in arguments {
        match argument {
            Argument::Scalar(Value::Error(error)) => return Err(error.clone()),
            Argument::Range(values) => {
                if let Some(Value::Error(error)) =
                    values.iter().find(|value| matches!(value, Value::Error(_)))
                {
                    return Err(error.clone());
                }
            }
            Argument::Scalar(_) | Argument::Missing => {}
        }
    }
    Ok(())
}

fn require_count(
    name: &str,
    arguments: &[Argument],
    minimum: usize,
    maximum: usize,
) -> CalcResult<()> {
    if (minimum..=maximum).contains(&arguments.len()) {
        Ok(())
    } else if minimum == maximum {
        Err(CalcError::value(format!(
            "{name} expects {minimum} arguments"
        )))
    } else {
        Err(CalcError::value(format!(
            "{name} expects {minimum} to {maximum} arguments"
        )))
    }
}

fn selected_if_value(argument: Argument) -> CalcResult {
    match argument {
        Argument::Scalar(Value::Error(error)) => Err(error),
        Argument::Scalar(value) => Ok(value),
        Argument::Missing => Ok(Value::from(0_i64)),
        Argument::Range(_) => Err(CalcError::value("IF branch must be a scalar")),
    }
}

fn if_prepared(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    require_count("IF", arguments, 1, 3)?;
    let condition = coerce_logical(required_scalar(&arguments[0], "IF condition")?, policy)?;
    if arguments.len() == 1 {
        return Ok(Value::Boolean(condition));
    }
    let selected = if condition { 1 } else { 2 };
    if selected >= arguments.len() {
        return Ok(Value::Boolean(false));
    }
    selected_if_value(arguments[selected].clone())
}

fn collect_numbers(arguments: &[Argument], policy: CoercionPolicy) -> CalcResult<Vec<Number>> {
    let mut numbers = Vec::new();
    for argument in arguments {
        match argument {
            Argument::Missing => return Err(CalcError::value("missing aggregate argument")),
            Argument::Scalar(value) => match value {
                Value::Number(number) => numbers.push(*number),
                Value::Empty => {}
                Value::Error(error) => return Err(error.clone()),
                Value::Boolean(_) | Value::Text(_) => {
                    numbers.push(coerce_number(value, policy)?);
                }
            },
            Argument::Range(values) => {
                for value in values {
                    match value {
                        Value::Number(number) => numbers.push(*number),
                        Value::Empty => {}
                        Value::Error(error) => return Err(error.clone()),
                        Value::Boolean(_) | Value::Text(_)
                            if policy == CoercionPolicy::OpenFormula => {}
                        Value::Boolean(_) | Value::Text(_) => {
                            return Err(CalcError::value(
                                "range contains a non-numeric value under strict coercion",
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(numbers)
}

fn strict_requires_argument(
    name: &str,
    arguments: &[Argument],
    policy: CoercionPolicy,
) -> CalcResult<()> {
    if policy == CoercionPolicy::Strict && arguments.is_empty() {
        Err(CalcError::value(format!(
            "{name} requires at least one argument under strict coercion"
        )))
    } else {
        Ok(())
    }
}

fn sum(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    strict_requires_argument("SUM", arguments, policy)?;
    let numbers = collect_numbers(arguments, policy)?;
    if numbers.is_empty() {
        return Ok(Value::from(0_i64));
    }
    let all_integer = numbers.iter().all(|number| number.as_i64().is_some());
    if all_integer {
        let mut total = Number::from_i64(0);
        for number in numbers {
            total = total.checked_add(number)?;
        }
        return Ok(Value::Number(total));
    }
    let mut total = 0.0;
    let mut compensation = 0.0;
    for number in numbers {
        let value = number.as_f64();
        let adjusted = value - compensation;
        let next = total + adjusted;
        compensation = (next - total) - adjusted;
        total = next;
        if !total.is_finite() {
            return Err(CalcError::num("SUM overflow"));
        }
    }
    Ok(Value::Number(Number::try_from_f64(total)?))
}

fn min(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    strict_requires_argument("MIN", arguments, policy)?;
    let numbers = collect_numbers(arguments, policy)?;
    let value = numbers
        .into_iter()
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal))
        .unwrap_or_else(|| Number::from_i64(0));
    Ok(Value::Number(value))
}

fn max(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    strict_requires_argument("MAX", arguments, policy)?;
    let numbers = collect_numbers(arguments, policy)?;
    let value = numbers
        .into_iter()
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal))
        .unwrap_or_else(|| Number::from_i64(0));
    Ok(Value::Number(value))
}

fn average(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    strict_requires_argument("AVERAGE", arguments, policy)?;
    let numbers = collect_numbers(arguments, policy)?;
    if numbers.is_empty() {
        return Err(CalcError::new(
            CalcErrorKind::DivZero,
            "AVERAGE has no numeric values",
        ));
    }
    let mut total = 0.0;
    let mut compensation = 0.0;
    for number in &numbers {
        let adjusted = number.as_f64() - compensation;
        let next = total + adjusted;
        compensation = (next - total) - adjusted;
        total = next;
    }
    let value = total / numbers.len() as f64;
    Ok(Value::Number(Number::try_from_f64(value)?))
}

fn abs(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    require_count("ABS", arguments, 1, 1)?;
    let value = coerce_number(required_scalar(&arguments[0], "ABS value")?, policy)?;
    Ok(Value::Number(value.checked_abs()?))
}

fn round(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    require_count("ROUND", arguments, 1, 2)?;
    let value = coerce_number(required_scalar(&arguments[0], "ROUND value")?, policy)?;
    let digits = if arguments.len() == 2 {
        let digits = coerce_number(required_scalar(&arguments[1], "ROUND digits")?, policy)?;
        let digits = digits
            .as_i64_exact()
            .ok_or_else(|| CalcError::value("ROUND digits must be an integer"))?;
        if !(-15..=15).contains(&digits) {
            return Err(CalcError::new(
                CalcErrorKind::Limit,
                "ROUND digits must be in -15..=15",
            ));
        }
        digits as i32
    } else {
        0
    };
    let factor = libm::pow(10.0, f64::from(digits.unsigned_abs()));
    let rounded = if digits >= 0 {
        stable_round_half_away(value.as_f64() * factor) / factor
    } else {
        stable_round_half_away(value.as_f64() / factor) * factor
    };
    Ok(Value::Number(Number::try_from_f64(rounded)?))
}

fn floor(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    round_to_multiple("FLOOR", arguments, policy, Direction::Floor)
}

fn ceiling(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    round_to_multiple("CEILING", arguments, policy, Direction::Ceiling)
}

#[derive(Clone, Copy)]
enum Direction {
    Floor,
    Ceiling,
}

fn round_to_multiple(
    name: &str,
    arguments: &[Argument],
    policy: CoercionPolicy,
    direction: Direction,
) -> CalcResult {
    require_count(name, arguments, 1, 3)?;
    let number = coerce_number(required_scalar(&arguments[0], "rounding value")?, policy)?;
    let raw_number = number.as_f64();
    let significance = if arguments.len() >= 2 {
        match &arguments[1] {
            Argument::Missing => {
                if raw_number < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            argument => coerce_number(required_scalar(argument, "significance")?, policy)?.as_f64(),
        }
    } else if raw_number < 0.0 {
        -1.0
    } else {
        1.0
    };
    let mode = if arguments.len() == 3 {
        match &arguments[2] {
            Argument::Missing => false,
            argument => !coerce_number(required_scalar(argument, "mode")?, policy)?.is_zero(),
        }
    } else {
        false
    };
    if raw_number == 0.0 || significance == 0.0 {
        return Ok(Value::Number(Number::try_from_f64(0.0)?));
    }
    if raw_number.is_sign_negative() != significance.is_sign_negative() {
        return Err(CalcError::num(format!(
            "{name} value and significance must have the same sign"
        )));
    }
    let unit = libm::fabs(significance);
    let (quotient, sign) = if mode {
        (
            libm::fabs(raw_number) / unit,
            if raw_number < 0.0 { -1.0 } else { 1.0 },
        )
    } else {
        (raw_number / unit, 1.0)
    };
    if !quotient.is_finite() {
        return Err(CalcError::num(format!("{name} quotient overflow")));
    }
    let nearest = libm::round(quotient);
    if libm::fabs(quotient - nearest) <= two_ulp_tolerance(quotient) {
        return Ok(Value::Number(Number::try_from_f64(raw_number)?));
    }
    let multiple = match (direction, mode) {
        (Direction::Ceiling, _) => libm::ceil(quotient),
        (Direction::Floor, _) => libm::floor(quotient),
    };
    let result = normalize_zero(multiple * unit * sign);
    Ok(Value::Number(Number::try_from_f64(result)?))
}

fn power(
    arguments: &[Argument],
    _context: &mut dyn EvalContext,
    policy: CoercionPolicy,
) -> CalcResult {
    require_count("POWER", arguments, 2, 2)?;
    let base = coerce_number(required_scalar(&arguments[0], "POWER base")?, policy)?;
    let exponent = coerce_number(required_scalar(&arguments[1], "POWER exponent")?, policy)?;
    let base = base.as_f64();
    let exponent = exponent.as_f64();
    if base == 0.0 && exponent < 0.0 {
        return Err(CalcError::new(
            CalcErrorKind::DivZero,
            "POWER cannot raise zero to a negative exponent",
        ));
    }
    if base == 0.0 && exponent == 0.0 {
        return Ok(Value::from(1_i64));
    }
    if base < 0.0 && libm::trunc(exponent) != exponent {
        return Err(CalcError::num(
            "POWER with a negative base requires an integer exponent",
        ));
    }
    Ok(Value::Number(Number::try_from_f64(libm::pow(
        base, exponent,
    ))?))
}

fn stable_round_half_away(value: f64) -> f64 {
    let truncated = libm::trunc(value);
    let fraction = value - truncated;
    if libm::fabs(libm::fabs(fraction) - 0.5) <= two_ulp_tolerance(value) {
        truncated + if value < 0.0 { -1.0 } else { 1.0 }
    } else {
        libm::round(value)
    }
}

fn two_ulp_tolerance(value: f64) -> f64 {
    let next = next_up(value);
    2.0 * libm::fabs(next - value)
}

fn next_up(value: f64) -> f64 {
    if value == f64::INFINITY {
        return value;
    }
    if value == -0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value >= 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}
