Number formatting.

This one uses a pattern string instead of the `format!` style.

```
use format_num_pattern::Locale::de_AT_euro;
use format_num_pattern as num;
use format_num_pattern::{NumberFormat, NumberSymbols};

// formats accordingly, uses the default symbols.
let s = num::format(4561.2234, "###,##0.00").expect("works");
assert_eq!(s, "  4,561.22");

// uses symbols
let sym = NumberSymbols::monetary(de_AT_euro);
let s = num::formats(4561.2234, "$ ###,##0.00", &sym).expect("works");
assert_eq!(s.as_str(), "€   4\u{202f}561,22");

// prepared format
let sym = NumberSymbols::monetary(de_AT_euro);
let m2 = NumberFormat::news("$ ###,##0.00", sym).expect("works");

let s = m2.fmt(4561.2234).expect("works");
assert_eq!(s.as_str(), "€   4\u{202f}561,22");

// postfix fmt using the FormatNumber trait
use format_num_pattern::FormatNumber;
println!("combined output: {}", 4561.2234f64.fmt(&m2));
```

The following patterns are recognized:

* `0` - digit or 0
* `9` - digit or space
* `#` - digit or sign or space
* `-` - sign; show space for positive
* `+` - sign; show '+' for positive and '-' for negative. not localized.
* `.` - decimal separator
* `:` - decimal separator, always shown
* `,` - grouping separator. Might be completely absent, if the FormatSymbols
  say so.
* `E` - upper case exponent
* `e` - lower case exponent
* ` ` - space can be used as separator
* '$' - currency. variable length output according to the currency-symbol.
* `\` - all ascii characters (ascii 32-128!) are reserved and must be escaped.
* `_` - other unicode characters can be used without escaping.

The formatting and parsing functions use [NumberSymbols] for localization.
The localization itself is provided by
[pure_rust_locales](https://crates.io/crates/pure-rust-locales).