# openformula-kernel

`openformula-kernel` is a deterministic, typed Rust calculation kernel shared by
Tanu Markdown, TSQ1, and Kitu. It implements a deliberately bounded subset of
the function semantics in OpenFormula 1.4. It is **not** a complete formula
parser, workbook engine, or OpenFormula-conforming evaluator.

The crate owns scalar values, numeric policy, coercion, typed calculation
errors, standard functions, an injectable evaluation context, and a registry
that keeps standard and namespaced extension functions separate. Callers keep
ownership of parsing, references, dependency graphs, persistence, and UI.

## Supported standard functions

The initial `0.1.0` API provides `SUM`, `MIN`, `MAX`, `AVERAGE`, `IF`, `ABS`,
`ROUND`, `FLOOR`, `CEILING`, and `POWER`. See
[`docs/compatibility.md`](docs/compatibility.md) for the normative section of
OpenFormula 1.4 and every intentional difference.

```rust
use openformula_kernel::{Argument, FunctionRegistry, PureContext, Value};

let registry = FunctionRegistry::standard();
let mut context = PureContext;
let result = registry.evaluate(
    "SUM",
    &[
        Argument::scalar(Value::from(2_i64)),
        Argument::scalar(Value::from(3_i64)),
    ],
    &mut context,
)?;
assert_eq!(result, Value::from(5_i64));
# Ok::<(), openformula_kernel::CalcError>(())
```

Use `FunctionRegistry::evaluate_lazy` for `IF` so the unselected branch is
never evaluated. Extensions must use a namespace such as `KITU.CLAMP`; the
crate includes `extensions::register_kitu_examples` as a dependency-light
example.

## Distribution and compatibility

- Releases use semantic versioning. A behavior change that can alter persisted
  calculation results requires at least a minor version before `1.0`.
- Consumers pin the same Git revision until a crates.io publication policy is
  adopted. Release tags use `vMAJOR.MINOR.PATCH`.
- The default `std` feature only adds `std::error::Error`; the calculation core
  works with `--no-default-features` and `alloc`, including WASM consumers.
- The stable public `Number` wrapper hides the current binary64 representation
  so a later decimal implementation does not force callers to rewrite APIs.

## Verification

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
```

See [`docs/numeric-policy.md`](docs/numeric-policy.md) for determinism and
rounding guarantees and [`docs/migration.md`](docs/migration.md) for consumer
adapter guidance.
