use format_num_pattern::{NumberFormat, NumberSymbols};
use rust_decimal::Decimal;

#[test]
fn test_1() {
    let sym = NumberSymbols::default();

    let f0 = NumberFormat::news("###,##0.##", sym).expect("fine");
    let f: f32 = f0.parse("     12.  ").expect("fine");
    dbg!(f);
    let f: Decimal = f0.parse("     12.  ").expect("fine");
    dbg!(f);
}
