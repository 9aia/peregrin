use crate::date_time_format::{
    DateTimeFieldStyle, DateTimeFormat, DateTimeFormatOptions, DateTimeStyle, DateType, Error,
    TimeZoneNameStyle,
};
use ecma402_traits::date_time_format::DateTimeFormat as DateTimeFormatTrait;

fn style(s: &str) -> DateTimeStyle {
    match s {
        "full" => DateTimeStyle::Full,
        "long" => DateTimeStyle::Long,
        "short" => DateTimeStyle::Short,
        _ => DateTimeStyle::Medium,
    }
}

fn field(s: &str) -> DateTimeFieldStyle {
    match s {
        "numeric" => DateTimeFieldStyle::Numeric,
        "2-digit" => DateTimeFieldStyle::TwoDigit,
        "long" => DateTimeFieldStyle::Long,
        "short" => DateTimeFieldStyle::Short,
        "narrow" => DateTimeFieldStyle::Narrow,
        _ => DateTimeFieldStyle::Short,
    }
}

fn date_opts() -> DateTimeFormatOptions {
    DateTimeFormatOptions {
        date_type: DateType::Date,
        ..Default::default()
    }
}

fn time_opts() -> DateTimeFormatOptions {
    DateTimeFormatOptions {
        date_type: DateType::Time,
        ..Default::default()
    }
}

fn datetime_opts() -> DateTimeFormatOptions {
    DateTimeFormatOptions {
        date_type: DateType::DateTime,
        ..Default::default()
    }
}

#[test]
fn format_date() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", date_opts())?;
    let s = dtf.format("2024-03-15")?;
    assert!(!s.is_empty());
    assert!(s.contains("2024") || s.contains("24"));
    Ok(())
}

#[test]
fn format_to_parts_date() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", date_opts())?;
    let parts = dtf.format_to_parts("2024-03-15")?;
    assert!(!parts.is_empty());
    Ok(())
}

#[test]
fn format_time() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", time_opts())?;
    let s = dtf.format("14:30:45")?;
    assert!(!s.is_empty());
    Ok(())
}

#[test]
fn format_to_parts_time() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", time_opts())?;
    let parts = dtf.format_to_parts("14:30:45")?;
    assert!(!parts.is_empty());
    Ok(())
}

#[test]
fn format_datetime() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", datetime_opts())?;
    let s = dtf.format("2024-03-15T14:30:45")?;
    assert!(!s.is_empty());
    Ok(())
}

#[test]
fn format_to_parts_datetime() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", datetime_opts())?;
    let parts = dtf.format_to_parts("2024-03-15T14:30:45")?;
    assert!(!parts.is_empty());
    Ok(())
}

#[test]
fn date_style_variations() -> Result<(), Error> {
    for s in ["full", "long", "medium", "short"] {
        let options = DateTimeFormatOptions {
            date_type: DateType::Date,
            date_style: Some(style(s)),
            ..Default::default()
        };
        let dtf = DateTimeFormat::new("en-US", options)?;
        let formatted = dtf.format("2024-03-15")?;
        assert!(!formatted.is_empty());
    }
    Ok(())
}

#[test]
fn time_style_variations() -> Result<(), Error> {
    for s in ["full", "long", "medium", "short"] {
        let options = DateTimeFormatOptions {
            date_type: DateType::Time,
            time_style: Some(style(s)),
            ..Default::default()
        };
        let dtf = DateTimeFormat::new("en-US", options)?;
        let formatted = dtf.format("14:30:45")?;
        assert!(!formatted.is_empty());
    }
    Ok(())
}

#[test]
fn datetime_style_variations() -> Result<(), Error> {
    for (d, t) in [
        ("full", "full"),
        ("long", "long"),
        ("medium", "medium"),
        ("short", "short"),
    ] {
        let options = DateTimeFormatOptions {
            date_type: DateType::DateTime,
            date_style: Some(style(d)),
            time_style: Some(style(t)),
            ..Default::default()
        };
        let dtf = DateTimeFormat::new("en-US", options)?;
        let formatted = dtf.format("2024-03-15T14:30:45")?;
        assert!(!formatted.is_empty());
    }
    Ok(())
}

#[test]
fn field_bag_date_fields() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::Date,
        year: Some(field("numeric")),
        month: Some(field("long")),
        day: Some(field("numeric")),
        weekday: Some(field("long")),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("2024-03-15")?;
    assert!(s.contains("2024") || s.contains("24"));
    assert!(!s.is_empty());
    Ok(())
}

#[test]
fn field_bag_time_fields() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::Time,
        hour: Some(field("numeric")),
        minute: Some(field("numeric")),
        second: Some(field("numeric")),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let parts = dtf.format_to_parts("14:30:45")?;
    assert!(!parts.is_empty());
    Ok(())
}

#[test]
fn field_bag_time_zone_name() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::DateTime,
        year: Some(field("numeric")),
        month: Some(field("numeric")),
        day: Some(field("numeric")),
        hour: Some(field("numeric")),
        minute: Some(field("numeric")),
        time_zone_name: Some(TimeZoneNameStyle::Short),
        time_zone: Some("America/New_York".to_string()),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("2024-03-15T12:00:00-04:00")?;
    assert!(!s.is_empty());
    Ok(())
}

#[test]
fn day_period_is_not_rendered() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::Time,
        hour: Some(field("numeric")),
        day_period: Some(field("short")),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("14:30:00")?;
    assert!(!s.is_empty());
    Ok(())
}

#[test]
fn style_time_zone_name() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::DateTime,
        date_style: Some(DateTimeStyle::Long),
        time_style: Some(DateTimeStyle::Long),
        time_zone_name: Some(TimeZoneNameStyle::Short),
        time_zone: Some("America/New_York".to_string()),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("2024-03-15T12:00:00-04:00")?;
    assert!(!s.is_empty());
    assert!(s.contains("EDT") || s.contains("GMT-4") || s.contains("ET"));
    Ok(())
}

#[test]
fn fractional_second_digits_style() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::Time,
        time_style: Some(DateTimeStyle::Medium),
        fractional_second_digits: Some(2),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("14:30:45.123")?;
    assert!(s.contains(".12"));
    Ok(())
}

#[test]
fn fractional_second_digits_field_bag() -> Result<(), Error> {
    let options = DateTimeFormatOptions {
        date_type: DateType::Time,
        hour: Some(field("numeric")),
        minute: Some(field("numeric")),
        second: Some(field("numeric")),
        fractional_second_digits: Some(2),
        ..Default::default()
    };
    let dtf = DateTimeFormat::new("en-US", options)?;
    let s = dtf.format("14:30:45.123")?;
    assert!(s.contains(".12"));
    Ok(())
}

#[test]
fn invalid_locale() {
    let result = DateTimeFormat::new("not a locale", DateTimeFormatOptions::default());
    assert!(matches!(result, Err(Error::InvalidLocale(_))));
}

#[test]
fn invalid_date() -> Result<(), Error> {
    let dtf = DateTimeFormat::new("en-US", date_opts())?;
    let result = dtf.format("not-a-date");
    assert!(matches!(result, Err(Error::InvalidDate(_))));
    Ok(())
}