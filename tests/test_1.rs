use format_num_pattern::{parse_sym, NumberFormat, NumberSymbols};
use pure_rust_locales::Locale;
use rust_decimal::Decimal;

#[test]
fn test_parse_zero_amount() {
    // This is successful: Ok(0.00)
    println!("{:?}", "0.00".parse::<Decimal>());
    // This is not successful: Err(Parse)
    println!(
        "{:?}",
        parse_sym::<Decimal>("0.00", &NumberSymbols::monetary(Locale::POSIX))
    );
    // Parses incorrectly: 0.1
    println!(
        "{:?}",
        parse_sym::<Decimal>("0.01", &NumberSymbols::monetary(Locale::POSIX))
    );
    // Panic! On unwrap()
    println!(
        "{:?}",
        parse_sym::<Decimal>("0.00", &NumberSymbols::monetary(Locale::POSIX))
    );
    // Panic! On unwrap()
    println!(
        "{:?}",
        parse_sym::<f32>("000000001.01", &NumberSymbols::monetary(Locale::POSIX))
    );
}
