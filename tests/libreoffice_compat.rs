//! Representative differential cases checked against LibreOffice Calc.

use openformula_kernel::{Argument, CalcErrorKind, FunctionRegistry, Number, PureContext, Value};

fn real(value: f64) -> Argument {
    Argument::scalar(Value::Number(
        Number::try_from_f64(value).expect("finite fixture"),
    ))
}

fn calculate(name: &str, arguments: &[Argument]) -> Result<f64, CalcErrorKind> {
    let value = FunctionRegistry::standard()
        .evaluate(name, arguments, &mut PureContext)
        .map_err(|error| error.kind())?;
    match value {
        Value::Number(number) => Ok(number.as_f64()),
        _ => panic!("fixture must return a number"),
    }
}

#[test]
fn decimal_multiple_rounding_matches_calc() {
    assert_eq!(calculate("FLOOR", &[real(0.3), real(0.1)]), Ok(0.3));
    assert_eq!(calculate("CEILING", &[real(0.14), real(0.01)]), Ok(0.14));
    assert_eq!(calculate("FLOOR", &[real(-2.3)]), Ok(-3.0));
    assert_eq!(calculate("CEILING", &[real(-2.3)]), Ok(-2.0));
    assert_eq!(
        calculate("FLOOR", &[real(-2.3), real(-1.0), Argument::scalar(1_i64)]),
        Ok(-2.0)
    );
    assert_eq!(
        calculate(
            "CEILING",
            &[real(-2.3), real(-1.0), Argument::scalar(1_i64)]
        ),
        Ok(-3.0)
    );
}

#[test]
fn halfway_rounding_matches_calc() {
    assert_eq!(calculate("ROUND", &[real(2.5)]), Ok(3.0));
    assert_eq!(calculate("ROUND", &[real(-2.5)]), Ok(-3.0));
    assert_eq!(
        calculate("ROUND", &[real(145.0), Argument::scalar(-1_i64)]),
        Ok(150.0)
    );
}

#[test]
fn power_domain_choices_are_explicit() {
    assert_eq!(
        calculate("POWER", &[Argument::scalar(0_i64), Argument::scalar(0_i64)]),
        Ok(1.0)
    );
    assert_eq!(
        calculate(
            "POWER",
            &[Argument::scalar(0_i64), Argument::scalar(-1_i64)]
        ),
        Err(CalcErrorKind::DivZero)
    );
    assert_eq!(
        calculate("POWER", &[Argument::scalar(-2_i64), real(0.5)]),
        Err(CalcErrorKind::Num)
    );
    assert_eq!(
        calculate(
            "POWER",
            &[Argument::scalar(2_i64), Argument::scalar(10_i64)]
        ),
        Ok(1024.0)
    );
}

#[test]
fn range_non_numbers_are_ignored() {
    assert_eq!(
        calculate(
            "AVERAGE",
            &[Argument::range([
                Value::from(10_i64),
                Value::from("text"),
                Value::Boolean(true),
                Value::Empty,
                Value::from(20_i64),
            ])]
        ),
        Ok(15.0)
    );
}
