use super::{Error, NumberFormat, NumberFormatOptions};
use ecma402_traits::number_format::{
    CompactDisplay, CurrencyDisplay, Notation, NumberFormat as NumberFormatTrait,
    NumberFormatPartType, NumberFormatStyle, RoundingMode, SignDisplay, UnitDisplay, UseGrouping,
};

fn en_decimal_opts() -> NumberFormatOptions {
    NumberFormatOptions {
        style: NumberFormatStyle::Decimal,
        ..Default::default()
    }
}

#[test]
fn decimal_grouping_and_fraction() -> Result<(), Error> {
    let nf = NumberFormat::new("en", en_decimal_opts())?;
    let s = nf.format(1234.56)?;
    assert!(s.contains(','), "grouping not present: {s}");
    assert!(s.contains('.'), "decimal not present: {s}");
    assert!(s.contains("56"), "fraction not present: {s}");
    Ok(())
}

#[test]
fn decimal_no_grouping() -> Result<(), Error> {
    let mut options = en_decimal_opts();
    options.use_grouping = UseGrouping::False;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(1234.56)?;
    assert!(!s.contains(','), "grouping should be disabled: {s}");
    Ok(())
}

#[test]
fn percent_basic() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Percent;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(0.05)?;
    assert!(s.contains('%'), "percent sign missing: {s}");
    assert!(s.starts_with('5'), "unexpected percent start: {s}");
    Ok(())
}

#[test]
fn currency_symbol() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Currency;
    options.currency = Some("USD".to_string());
    options.currency_display = CurrencyDisplay::Symbol;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(1234.5)?;
    assert!(s.starts_with('$'), "expected USD symbol: {s}");
    assert!(s.contains(','), "expected grouping: {s}");
    Ok(())
}

#[test]
fn unit_short_meter() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Unit;
    options.unit = Some("meter".to_string());
    options.unit_display = UnitDisplay::Short;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(1.0)?;
    assert!(s.contains('m'), "expected meter short unit: {s}");
    Ok(())
}

#[test]
fn unit_short_second() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Unit;
    options.unit = Some("second".to_string());
    options.unit_display = UnitDisplay::Short;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(2.0)?;
    assert!(s.contains('s'), "expected second short unit: {s}");
    Ok(())
}

#[test]
fn compact_short() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.notation = Notation::Compact;
    options.compact_display = CompactDisplay::Short;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(12345.0)?;
    assert!(s.contains('K'), "expected compact K: {s}");
    Ok(())
}

#[test]
fn scientific_notation() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.notation = Notation::Scientific;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(12345.0)?;
    assert!(s.contains('×'), "expected × separator: {s}");
    assert!(s.contains('1'), "expected mantissa: {s}");
    Ok(())
}

#[test]
fn engineering_notation() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.notation = Notation::Engineering;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(12345.0)?;
    assert!(s.contains('×'), "expected × separator: {s}");
    Ok(())
}

#[test]
fn decimal_to_parts() -> Result<(), Error> {
    let nf = NumberFormat::new("en", en_decimal_opts())?;
    let parts = nf.format_to_parts(1234.56)?;
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::Integer)
    );
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::Fraction)
    );
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::Decimal)
    );
    Ok(())
}

#[test]
fn scientific_to_parts() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.notation = Notation::Scientific;
    let nf = NumberFormat::new("en", options)?;
    let parts = nf.format_to_parts(12345.0)?;
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::Literal && p.value == "×10")
    );
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::Integer)
    );
    Ok(())
}

#[test]
fn digit_options_padding() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.minimum_integer_digits = Some(5);
    options.minimum_fraction_digits = Some(2);
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(12.3)?;
    assert!(s.contains("00,012"), "expected integer padding: {s}");
    assert!(s.ends_with(".30"), "expected fraction padding: {s}");
    Ok(())
}

#[test]
fn sign_display_always() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.sign_display = SignDisplay::Always;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(1.0)?;
    assert!(s.starts_with('+'), "expected plus sign: {s}");
    Ok(())
}

#[test]
fn rounding_mode_ceil() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.maximum_fraction_digits = Some(2);
    options.rounding_mode = RoundingMode::Ceil;
    let nf = NumberFormat::new("en", options)?;
    let s = nf.format(1.231)?;
    assert!(s.ends_with(".24"), "expected ceil rounding: {s}");
    Ok(())
}

#[test]
fn error_invalid_locale() {
    let result = NumberFormat::new("not_a_locale", NumberFormatOptions::default());
    assert!(matches!(result, Err(Error::InvalidLocale(_))));
}

#[test]
fn error_missing_currency() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Currency;
    let nf = NumberFormat::new("en", options)?;
    let result = nf.format(1.0);
    assert!(matches!(result, Err(Error::InvalidArgument(_))));
    Ok(())
}

#[test]
fn error_invalid_unit() -> Result<(), Error> {
    let mut options = NumberFormatOptions::default();
    options.style = NumberFormatStyle::Unit;
    options.unit = Some("not-a-real-unit".to_string());
    let nf = NumberFormat::new("en", options)?;
    let result = nf.format(1.0);
    assert!(matches!(result, Err(Error::InvalidUnit(_))));
    Ok(())
}

#[test]
fn parts_maintain_value_types() -> Result<(), Error> {
    let nf = NumberFormat::new("en", en_decimal_opts())?;
    let parts = nf.format_to_parts(-1234.56)?;
    assert!(
        parts
            .iter()
            .any(|p| p.part_type == NumberFormatPartType::MinusSign
                || p.part_type == NumberFormatPartType::PlusSign)
    );
    let combined: String = parts.iter().map(|p| p.value.as_str()).collect();
    assert!(combined.contains("-"));
    Ok(())
}
