# OpenFormula 1.4 compatibility matrix

This crate is an **OpenFormula 1.4 function-semantics subset**, not an
OpenFormula-conforming evaluator. Section references below refer to OASIS
OpenDocument 1.4, Part 4 (OpenFormula).

| Function | OpenFormula 1.4 | Status | Intentional differences or decisions |
| --- | --- | --- | --- |
| `SUM` | 6.16.61 | Implemented | Locale-neutral scalar text parsing is selected from the implementation-defined conversion choices. Range text, logical, and empty values are ignored with `OpenFormula` coercion. |
| `MIN` | 6.18.48 | Implemented | Returns zero when no numeric value is present, including zero arguments. |
| `MAX` | 6.18.45 | Implemented | Returns zero when no numeric value is present. The standard explicitly specifies non-number filtering but leaves the empty result implicit; zero matches common evaluator behavior and `MIN`. |
| `AVERAGE` | 6.18.3 | Implemented | No numeric values returns `#DIV/0!`. |
| `IF` | 6.15.4 | Implemented | All one-, two-, and three-argument forms are supported. `evaluate_lazy` is the conforming entry point because it only evaluates the selected branch. |
| `ABS` | 6.16.2 | Implemented | Integer minimum overflow returns `#NUM!` rather than promoting silently. |
| `ROUND` | 6.17.5 | Bounded | Halfway values round away from zero. `Digits` is limited to `-15..=15` to keep results deterministic and bounded. |
| `FLOOR` | 6.17.3 | Implemented | Supports optional significance and mode. Decimal exact-multiple detection uses a local two-ULP boundary correction. |
| `CEILING` | 6.17.1 | Implemented | Supports optional significance and mode. Decimal exact-multiple detection uses a local two-ULP boundary correction. |
| `POWER` | 6.16.46 | Implemented | `POWER(0,0)` returns `1`; zero to a negative exponent returns `#DIV/0!`; a negative base with a non-integer exponent returns `#NUM!`. |

`CoercionPolicy::OpenFormula` is the standard-facing policy. It converts direct
logical values to `0` or `1`, treats direct empty values as zero, and parses
direct text as a locale-neutral finite number. Values originating in a range
follow NumberSequence rules: only numbers participate and errors propagate.

`CoercionPolicy::Strict` is an explicit host compatibility policy. It rejects
text, logical, and empty values wherever a number or logical is required. Tanu
Markdown uses it to retain its pre-kernel typed Formula behavior. Results from
that policy are therefore not claims of OpenFormula coercion compatibility.

## Error mapping

The kernel uses typed errors corresponding to `#NULL!`, `#DIV/0!`, `#VALUE!`,
`#REF!`, `#NAME?`, `#NUM!`, and `#N/A`, plus bounded-runtime `#LIMIT!` and
host-capability `#UNSUPPORTED!` errors. Unless a function explicitly handles an
error, the leftmost input error propagates.

## Differential cases

`tests/libreoffice_compat.rs` records representative values checked against
LibreOffice Calc for positive and negative halfway rounding, decimal
significance boundaries, range coercion, empty aggregates, and power domain
errors. The `ROUND` digit bound and `Strict` coercion policy are documented
deviations and are tested separately.
