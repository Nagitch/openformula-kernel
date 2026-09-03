//! Public API and policy contract tests.

use core::cell::Cell;

use openformula_kernel::extensions::register_kitu_examples;
use openformula_kernel::{
    Argument, CalcError, CalcErrorKind, CalcResult, CoercionPolicy, EvalContext, FunctionNamespace,
    FunctionRegistry, Number, PureContext, Value,
};

fn real(value: f64) -> Value {
    Value::Number(Number::try_from_f64(value).expect("finite fixture"))
}

fn evaluate(registry: &FunctionRegistry, name: &str, arguments: &[Argument]) -> Value {
    registry
        .evaluate(name, arguments, &mut PureContext)
        .expect("calculation succeeds")
}

fn as_f64(value: Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64(),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn standard_registry_exposes_compatibility_metadata() {
    let registry = FunctionRegistry::standard();
    let names = registry
        .all_metadata()
        .map(|metadata| metadata.name())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 10);
    let round = registry.metadata("round").expect("ROUND metadata");
    assert_eq!(round.openformula_reference(), Some("6.17.5"));
    assert_eq!(round.namespace(), &FunctionNamespace::Standard);
}

#[test]
fn aggregate_sequence_conversion_distinguishes_scalars_and_ranges() {
    let registry = FunctionRegistry::standard();
    let result = evaluate(
        &registry,
        "sum",
        &[
            Argument::scalar("2.5"),
            Argument::scalar(true),
            Argument::range([
                Value::from(4_i64),
                Value::from("ignored"),
                Value::Boolean(true),
                Value::Empty,
            ]),
        ],
    );
    assert_eq!(as_f64(result), 7.5);

    let strict = FunctionRegistry::standard_with_policy(CoercionPolicy::Strict);
    let error = strict
        .evaluate(
            "SUM",
            &[Argument::range([Value::from(1_i64), Value::from("bad")])],
            &mut PureContext,
        )
        .expect_err("strict range rejects text");
    assert_eq!(error.kind(), CalcErrorKind::Value);
}

#[test]
fn aggregate_empty_and_integer_results_are_stable() {
    let registry = FunctionRegistry::standard();
    assert_eq!(evaluate(&registry, "SUM", &[]), Value::from(0_i64));
    assert_eq!(evaluate(&registry, "MIN", &[]), Value::from(0_i64));
    assert_eq!(evaluate(&registry, "MAX", &[]), Value::from(0_i64));
    assert_eq!(
        evaluate(
            &registry,
            "SUM",
            &[Argument::range([Value::from(2_i64), Value::from(3_i64)])]
        ),
        Value::from(5_i64)
    );
    let error = registry
        .evaluate(
            "AVERAGE",
            &[Argument::range([Value::Empty])],
            &mut PureContext,
        )
        .expect_err("empty average");
    assert_eq!(error.kind(), CalcErrorKind::DivZero);
}

#[test]
fn if_lazy_evaluates_only_the_selected_branch() {
    let registry = FunctionRegistry::standard();
    let visited_true = Cell::new(0);
    let visited_false = Cell::new(0);
    let result = registry
        .evaluate_lazy(
            "IF",
            3,
            |index| match index {
                0 => Ok(Argument::scalar(true)),
                1 => {
                    visited_true.set(visited_true.get() + 1);
                    Ok(Argument::scalar(42_i64))
                }
                2 => {
                    visited_false.set(visited_false.get() + 1);
                    Err(CalcError::new(CalcErrorKind::DivZero, "unselected"))
                }
                _ => unreachable!(),
            },
            &mut PureContext,
        )
        .expect("selected branch succeeds");
    assert_eq!(result, Value::from(42_i64));
    assert_eq!(visited_true.get(), 1);
    assert_eq!(visited_false.get(), 0);
}

#[test]
fn errors_propagate_left_to_right() {
    let registry = FunctionRegistry::standard();
    let first = CalcError::new(CalcErrorKind::Ref, "first");
    let second = CalcError::new(CalcErrorKind::Num, "second");
    let error = registry
        .evaluate(
            "SUM",
            &[
                Argument::scalar(Value::Error(first.clone())),
                Argument::range([Value::Error(second)]),
            ],
            &mut PureContext,
        )
        .expect_err("error propagates");
    assert_eq!(error, first);
}

#[test]
fn namespaced_extensions_cannot_replace_standard_functions() {
    let mut registry = FunctionRegistry::standard();
    register_kitu_examples(&mut registry).expect("extension registration");
    assert_eq!(
        evaluate(
            &registry,
            "kitu.clamp",
            &[
                Argument::scalar(12_i64),
                Argument::scalar(0_i64),
                Argument::scalar(10_i64),
            ],
        ),
        Value::from(10_i64)
    );
    let metadata = registry.metadata("KITU.CLAMP").expect("extension metadata");
    assert_eq!(
        metadata.namespace(),
        &FunctionNamespace::Extension("KITU".into())
    );
}

#[test]
fn extensions_receive_only_injected_impure_capabilities() {
    struct FixedContext;

    impl EvalContext for FixedContext {
        fn random_unit(&mut self) -> CalcResult<Number> {
            Number::try_from_f64(0.25)
        }
    }

    fn injected_random(
        arguments: &[Argument],
        context: &mut dyn EvalContext,
        _policy: CoercionPolicy,
    ) -> CalcResult {
        assert!(arguments.is_empty());
        Ok(Value::Number(context.random_unit()?))
    }

    let mut registry = FunctionRegistry::standard();
    registry
        .register_extension(
            "TEST",
            "RANDOM",
            "test-only injected random source",
            injected_random,
        )
        .expect("extension registration");
    let value = registry
        .evaluate("TEST.RANDOM", &[], &mut FixedContext)
        .expect("context is injected");
    assert_eq!(as_f64(value), 0.25);

    let error = registry
        .evaluate("TEST.RANDOM", &[], &mut PureContext)
        .expect_err("pure context rejects random access");
    assert_eq!(error.kind(), CalcErrorKind::Unsupported);
}

#[test]
fn number_rejects_non_finite_values_and_normalizes_negative_zero() {
    assert_eq!(
        Number::try_from_f64(f64::NAN)
            .expect_err("NaN rejected")
            .kind(),
        CalcErrorKind::Num
    );
    let zero = Number::try_from_f64(-0.0).expect("zero accepted");
    assert_eq!(zero.as_f64().to_bits(), 0.0_f64.to_bits());
}

#[test]
fn scalar_numeric_functions_cover_boundaries() {
    let registry = FunctionRegistry::standard();
    assert_eq!(
        evaluate(&registry, "ABS", &[Argument::scalar(-3_i64)]),
        Value::from(3_i64)
    );
    let overflow = registry
        .evaluate("ABS", &[Argument::scalar(i64::MIN)], &mut PureContext)
        .expect_err("integer abs overflow");
    assert_eq!(overflow.kind(), CalcErrorKind::Num);

    assert_eq!(
        as_f64(evaluate(
            &registry,
            "ROUND",
            &[Argument::scalar(real(1.005)), Argument::scalar(2_i64)]
        )),
        1.01
    );
    assert_eq!(
        as_f64(evaluate(
            &registry,
            "ROUND",
            &[Argument::scalar(real(-2.5))]
        )),
        -3.0
    );
}
