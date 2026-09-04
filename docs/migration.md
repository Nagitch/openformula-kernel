# Consumer migration

All consumers should pin one `openformula-kernel` version or Git revision and
keep parser- or storage-specific conversion in a small adapter.

## Tanu Markdown

Tanu retains Formula parsing, A1/range/header references, dependency ordering,
and source diagnostics. Its adapter converts `DataScalar` and flattened ranges
to kernel `Value` and `Argument`, evaluates standard functions with
`CoercionPolicy::Strict`, then maps typed calculation errors back to a source
span. `IF` uses the lazy registry API.

## TSQ1

TSQ1 exposes the no-std-compatible values and registry at its runtime boundary.
The sequence format, timing conversion, serialization, and event references
remain owned by TSQ1.

## Kitu

Kitu constructs the standard registry and explicitly installs namespaced Kitu
extensions. Runtime triggers, scripting, application state, and presentation
remain owned by Kitu.

## Change control

Before upgrading the pinned revision, each consumer runs the shared contract
cases for coercion, rounding, error propagation, and namespaced lookup. Any
intentional result change is recorded in both this compatibility matrix and the
consumer release notes. Syntax and storage migrations are never inferred from a
kernel upgrade.
