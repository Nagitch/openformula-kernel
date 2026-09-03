# Numeric and determinism policy

## Representation

`Number` is a public wrapper with a private representation. Version `0.1.0`
preserves signed 64-bit integers when possible and otherwise stores a finite
IEEE-754 binary64 value. Callers cannot depend on that representation.

- NaN and positive or negative infinity are rejected as `#NUM!`.
- Integer arithmetic that exceeds `i64` returns `#NUM!`.
- Floating-point overflow returns `#NUM!`.
- IEEE-754 underflow is accepted and may produce zero or a subnormal value.
- Negative zero is normalized to positive zero at every public boundary.
- Display formatting is intentionally outside this crate.

## Rounding

`ROUND` follows OpenFormula's halfway-away-from-zero rule. `FLOOR` and
`CEILING` implement the OpenFormula significance and mode rules rather than the
similarly named two-argument UI variants found in some spreadsheet products.

Binary floating-point can place a mathematical decimal multiple immediately on
the wrong side of an integer boundary (`0.3 / 0.1`, for example). Only the
multiple-rounding boundary check applies tolerance: a quotient within two ULPs
of an integer is treated as that integer. There is no global epsilon and normal
comparisons remain exact.

## Native and WASM

For identical input bits, value conversion, aggregates, `ABS`, `ROUND`,
`FLOOR`, `CEILING`, and conditional selection are expected to be bit-identical
across supported native and `wasm32-unknown-unknown` builds. `POWER` uses the
pure-Rust `libm` implementation; its documented contract is the correctly
bounded finite result with comparison at four ULPs in differential tests.

CI checks both the default and `no_std` builds. A WASM target check is included
in the repository workflow. Host adapters are responsible for rejecting values
that cannot be represented in their storage model.
