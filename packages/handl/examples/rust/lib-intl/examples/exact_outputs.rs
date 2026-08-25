use icu::decimal::DecimalFormatter;
use icu::decimal::input::Decimal;
use writeable::Writeable;
use icu::locale::Locale;

fn fmt(n: f64, locale: &str) -> String {
    let loc: Locale = locale.parse().unwrap();
    let f = DecimalFormatter::try_new(loc.into(), Default::default()).unwrap();
    let d = if n == 0.0 { Decimal::from(0_i64) } else { Decimal::try_from_str(&format!("{}", n)).unwrap() };
    f.format(&d).write_to_string().into_owned()
}

fn main() {
    println!("en 42: [{}]", fmt(42.0, "en"));
    println!("en 1234567: [{}]", fmt(1234567.0, "en"));
    println!("en 1234567.89: [{}]", fmt(1234567.89, "en"));
    println!("en 0: [{}]", fmt(0.0, "en"));
    println!("en -1234: [{}]", fmt(-1234.0, "en"));
    println!("pt-BR 1234567.89: [{}]", fmt(1234567.89, "pt-BR"));
    println!("de 1234567.89: [{}]", fmt(1234567.89, "de"));
    println!("fr 1234567.89: [{}]", fmt(1234567.89, "fr"));
    println!("bn 1000007: [{}]", fmt(1000007.0, "bn"));
}
