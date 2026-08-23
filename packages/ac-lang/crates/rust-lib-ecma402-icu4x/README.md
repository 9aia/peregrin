# `ecma402-icu4x`

ICU4X-backed concrete implementations of the `ecma402-traits` ECMA-402-style APIs.

This crate provides real formatting, collation, plural rules and locale services
on top of ICU4X. Every formatter has two constructors:

- `new` uses the compiled CLDR data bundled with `icu`.
- `with_blob` accepts an `icu_provider_blob::BlobDataProvider` for custom data.

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

## Using the API

Import the module and the relevant trait, construct an instance, and call the
ECMA-402 style method:

```rust
use ecma402_icu4x::plural_rules::{Options, PluralRules};
use ecma402_traits::plural_rules::{PluralCategory, PluralRules as _};

let rules = PluralRules::new("en", Options::default()).unwrap();

assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
assert_eq!(rules.select(5.0).unwrap(), PluralCategory::Other);
```

For custom CLDR data, use the `with_blob` constructor:

```rust
use ecma402_icu4x::plural_rules::{Options, PluralRules};
use ecma402_traits::plural_rules::{PluralCategory, PluralRules as _};
use icu_provider_blob::BlobDataProvider;

let provider: BlobDataProvider = /* load a data blob */;
let rules = PluralRules::with_blob("en", Options::default(), &provider).unwrap();

assert_eq!(rules.select(1.0).unwrap(), PluralCategory::One);
```

## Testing

The crate includes unit tests under `src/` for the implemented modules. Run them
with:

```sh
cargo test -p ecma402-icu4x
```

## License

See the workspace license file.
