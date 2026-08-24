# `ecma402-traits`

Platform-agnostic ECMA-402-style trait definitions for Rust.

This crate contains only the interface surface for ECMA-402 APIs. Concrete
backends (for example, the ICU4X-backed `ecma402-icu4x`) provide the
implementations.

## Modules

The public modules mirror the ECMA-402 `Intl` constructors:

- `collator` — `Intl.Collator`
- `date_time_format` — `Intl.DateTimeFormat`
- `display_names` — `Intl.DisplayNames`
- `list_format` — `Intl.ListFormat`
- `locale` — `Intl.Locale`
- `locale_match` — `Intl.getCanonicalLocales` / `Intl.supportedLocalesOf`
- `number_format` — `Intl.NumberFormat`
- `plural_rules` — `Intl.PluralRules`
- `relative_time` — `Intl.RelativeTimeFormat`
- `segmenter` — `Intl.Segmenter`

## Using the traits

Import the modules you need and implement the traits for your backend type.
The `Error` associated type is left opaque so each backend can expose its own
failure mode.

```rust
use ecma402_traits::plural_rules::{PluralRules, PluralRulesOptions, PluralRuleType, PluralCategory};

#[derive(Debug)]
struct SimpleRules;

impl PluralRules for SimpleRules {
    type Error = std::convert::Infallible;

    fn select(&self, n: f64) -> Result<PluralCategory, Self::Error> {
        Ok(if n == 1.0 { PluralCategory::One } else { PluralCategory::Other })
    }
}

let rules = SimpleRules;
assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
```

## Testing

The crate includes integration tests under `tests/` that exercise the public
trait surface with a trivial mock backend. Run them with:

```sh
cargo test -p ecma402-traits
```

## License

See the workspace license file.
