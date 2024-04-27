#![doc = include_str!("../readme.md")]

pub use pure_rust_locales::Locale;

use pure_rust_locales::locale_match;
use rust_decimal::Decimal;
use std::fmt;
use std::fmt::{Debug, Display, Error as FmtError, Formatter, LowerExp, Write as FmtWrite};
use std::str::{from_utf8_unchecked, FromStr};

/// Symbols for number formatting.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NumberSymbols {
    /// Decimal separator
    pub decimal_sep: char,
    /// Decimal grouping
    pub decimal_grp: Option<char>,
    /// Minus sign
    pub negative_sym: char,
    /// Plus sign
    pub positive_sym: char,
    /// Exponent
    pub exponent_upper_sym: char,
    /// Exponent
    pub exponent_lower_sym: char,
    /// Currency
    pub currency_sym: CurrencySym,
    // todo: zero-digit, infinity, nan
}

impl Default for NumberSymbols {
    fn default() -> Self {
        Self::new()
    }
}

impl NumberSymbols {
    pub const fn new() -> Self {
        Self {
            decimal_sep: '.',
            decimal_grp: Some(','),
            negative_sym: '-',
            positive_sym: ' ',
            exponent_upper_sym: 'E',
            exponent_lower_sym: 'e',
            currency_sym: CurrencySym::new("$"),
        }
    }

    /// Uses the locale information provided by `pure_rust_locales`.
    ///
    /// This function sets
    /// * decimal_sep to LC_NUMERIC::DECIMAL_POINT,
    /// * decimal_grp to LC_NUMERIC::THOUSANDS_SEP
    /// Fills the rest with defaults.
    pub fn numeric(locale: Locale) -> Self {
        Self {
            decimal_sep: first_or(locale_match!(locale => LC_NUMERIC::DECIMAL_POINT), '.'),
            decimal_grp: first_opt(locale_match!(locale => LC_NUMERIC::THOUSANDS_SEP)),
            negative_sym: '-',
            positive_sym: ' ',
            exponent_upper_sym: 'E',
            exponent_lower_sym: 'e',
            currency_sym: CurrencySym::new("$"),
        }
    }

    /// Uses the locale information provided by `pure_rust_locales`.
    ///
    /// This function sets
    /// * decimal_sep to LC_MONETARY::MON_DECIMAL_POINT,
    /// * decimal_grp to LC_MONETARY::MON_THOUSANDS_SEP
    /// * negative_sym to LC_MONETARY::NEGATIVE_SIGN
    /// * positive_sym to LC_MONETARY::POSITIVE_SIGN
    /// * currency_sym to LC_MONETARY::CURRENCY_SYMBOL
    /// Fills the rest with defaults.
    pub fn monetary(locale: Locale) -> Self {
        Self {
            decimal_sep: first_or(locale_match!(locale => LC_MONETARY::MON_DECIMAL_POINT), '.'),
            decimal_grp: first_opt(locale_match!(locale => LC_MONETARY::MON_THOUSANDS_SEP)),
            negative_sym: first_or(locale_match!(locale => LC_MONETARY::NEGATIVE_SIGN), '-'),
            positive_sym: first_or(locale_match!(locale => LC_MONETARY::POSITIVE_SIGN), ' '),
            exponent_upper_sym: 'E',
            exponent_lower_sym: 'e',
            currency_sym: CurrencySym::new(locale_match!(locale => LC_MONETARY::CURRENCY_SYMBOL)),
        }
    }

    /// Uses the locale information provided by `pure_rust_locales`.
    ///
    /// This function sets
    /// * decimal_sep to LC_MONETARY::MON_DECIMAL_POINT,
    /// * decimal_grp to LC_MONETARY::MON_THOUSANDS_SEP
    /// * negative_sym to LC_MONETARY::NEGATIVE_SIGN
    /// * positive_sym to LC_MONETARY::POSITIVE_SIGN
    /// * currency_sym to LC_MONETARY::INT_CURR_SYMBOL
    /// Fills the rest with defaults.
    pub fn int_monetary(locale: Locale) -> Self {
        Self {
            decimal_sep: first_or(locale_match!(locale => LC_MONETARY::MON_DECIMAL_POINT), '.'),
            decimal_grp: first_opt(locale_match!(locale => LC_MONETARY::MON_THOUSANDS_SEP)),
            negative_sym: first_or(locale_match!(locale => LC_MONETARY::NEGATIVE_SIGN), '-'),
            positive_sym: first_or(locale_match!(locale => LC_MONETARY::POSITIVE_SIGN), ' '),
            exponent_upper_sym: 'E',
            exponent_lower_sym: 'e',
            currency_sym: CurrencySym::new(locale_match!(locale => LC_MONETARY::INT_CURR_SYMBOL)),
        }
    }
}

// first char or default
#[inline]
fn first_or(s: &str, default: char) -> char {
    s.chars().next().unwrap_or(default)
}

// first char or default
#[inline]
fn first_opt(s: &str) -> Option<char> {
    s.chars().next()
}

/// Currency symbol.
/// Const constructable short inline string.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CurrencySym {
    len: u8,
    sym: [u8; 16],
}

impl Debug for CurrencySym {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CurrencySym")
            .field("len", &self.len)
            .field("sym", &self.as_str())
            .finish()
    }
}

impl CurrencySym {
    /// New currency symbol.
    pub const fn new(src: &str) -> Self {
        let mut sym = [0u8; 16];

        let src = src.as_bytes();
        let src_len = src.len();

        let mut i = 0;
        while i < src_len && i < 16 {
            sym[i] = src[i];
            i += 1;
        }

        CurrencySym {
            len: src_len as u8,
            sym,
        }
    }

    /// Convert back to &str
    pub fn as_str(&self) -> &str {
        // Safety:
        // Copied from &str and never modified.
        unsafe { from_utf8_unchecked(&self.sym[..self.len as usize]) }
    }

    /// Symbol len.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Symbol empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Display for CurrencySym {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'a> From<&'a str> for CurrencySym {
    fn from(value: &'a str) -> Self {
        CurrencySym::new(value)
    }
}

/// Number mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Integer,
    Fraction,
    Exponent,
}

/// Tokens for the format.
///
/// Digit0, Digit, Numeric, NumericOpt, GroupingSep hold an digit-index.
/// Depending on mode that's the index into the integer, fraction or exponent part of
/// the number.
///
/// Numeric has an extra flag, to mark if a sign at this position is possible.
/// Next to a grouping separator there can be no sign, it will be at the position
/// of the grouping separator.
#[allow(variant_size_differences)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    /// Mask char "0". Digit or 0
    Digit0(Mode, u32),
    /// Mask char "9". Digit or space
    Digit(Mode, u32),
    /// Mask char "#". Digit or sign or space
    Numeric(Mode, u32, bool),
    /// Mask char "-". Integer sign.
    SignInt,
    /// Mask char "+". Integer sign.
    PlusInt,
    /// Mask char ".". Decimal separator.
    DecimalSep,
    /// Mask char ":". Decimal separator, always displayed.
    DecimalSepAlways,
    /// Mask char ",". Grouping separator.
    GroupingSep(u32, bool),
    /// Mask char "E". Exponent separator.
    ExponentUpper,
    /// Mask char "e". Exponent separator.
    ExponentLower,
    /// Mask char "-". Exponent sign.
    SignExp,
    /// Mask char "+". Exponent sign.
    PlusExp,
    /// Mask char "$". Currency. Variable length.
    Currency,
    /// Other separator char to output literally. May be escaped with '\\'.
    Separator(char),
}

/// Holds the pattern for the number format and some additional data.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    /// Minimum position where a sign can be placed. Just left of a `Token::Digit0`
    min_int_sign: u32,
    /// Number of integer digits.
    len_int: u32,

    /// Decides which std-format is used. If true it's `{:e}` otherwise plain `{}`
    has_exp: bool,
    /// Has an exponent with a '0' pattern.
    has_exp_0: bool,
    /// Minimum position where a sign can be placed. Just left of a `Token::Digit0`
    min_exp_sign: u32,
    /// Number of exponent digits
    len_exp: u32,

    /// Has a fraction with a '0' pattern.
    has_frac_0: bool,
    /// The required precision for this format. Is used for the underlying std-format.
    len_frac: u8,

    /// Tokens.
    tok: Vec<Token>,
    /// Symbols.
    sym: NumberSymbols,
}

/// Errors
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NumberFmtError {
    /// General formatting error. Mostly from `write!()`
    Fmt,
    /// Integer len of the source is too long.
    FmtLenInt,
    /// Exponent len of the source is too long.
    FmtLenExp,
    /// Number is negative, but there is no place to show.
    FmtNoSign,
    /// Exponent is negative, but there is no place to show.
    FmtNoExpSign,
    /// General parse error. Mostly from `FromStr::parse()`
    Parse,
    /// Misplaced decimal separator in the pattern. Invalid decimal separator when parsing.
    ParseInvalidDecimalSep,
    /// Invalid sign in the pattern. Invalid sign when parsing.
    ParseInvalidSign,
    /// Invalid exponent in the pattern. Invalid exponent when parsing.
    ParseInvalidExp,
    /// Invalid exp sign in the pattern. Invalid exp sign when parsing.
    ParseInvalidExpSign,
    /// Unescaped char in the pattern.
    ParseUnescaped,
    /// Invalid digit when parsing.
    ParseInvalidDigit,
    /// Invalid grp sep when parsing.
    ParseInvalidGroupingSep,
    /// Invalid currency symbol when parsing.
    ParseInvalidCurrency,
    /// Invalid separator when parsing.
    ParseInvalidSeparator,
}

impl std::error::Error for NumberFmtError {}

impl Display for NumberFmtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<FmtError> for NumberFmtError {
    fn from(_: FmtError) -> Self {
        NumberFmtError::Fmt
    }
}

impl Display for NumberFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for t in &self.tok {
            match t {
                Token::Digit0(_, _) => write!(f, "0")?,
                Token::Digit(_, _) => write!(f, "9")?,
                Token::Numeric(_, _, _) => write!(f, "#")?,
                Token::SignInt => write!(f, "-")?,
                Token::PlusInt => write!(f, "-")?,
                Token::DecimalSep => write!(f, ".")?,
                Token::DecimalSepAlways => write!(f, ":")?,
                Token::GroupingSep(_, _) => write!(f, ",")?,
                Token::ExponentUpper => write!(f, "E")?,
                Token::ExponentLower => write!(f, "e")?,
                Token::SignExp => write!(f, "-")?,
                Token::PlusExp => write!(f, "+")?,
                Token::Currency => write!(f, "$")?,
                Token::Separator(c) => {
                    if *c < '\u{0100}' {
                        write!(f, "\\ ")?;
                    }
                    write!(f, "{}", *c)?;
                }
            }
        }
        Ok(())
    }
}

impl NumberFormat {
    /// New format from pattern.
    pub fn new<S: AsRef<str>>(pattern: S) -> Result<Self, NumberFmtError> {
        let tok = Self::parse_tokens(pattern.as_ref())?;
        Self::news_tok(tok, NumberSymbols::new())
    }

    /// New format from pattern + symbols
    pub fn news<S: AsRef<str>>(pattern: S, sym: NumberSymbols) -> Result<Self, NumberFmtError> {
        let tok = Self::parse_tokens(pattern.as_ref())?;
        Self::news_tok(tok, sym)
    }

    /// New format from token-array.
    fn news_tok(mut pattern: Vec<Token>, sym: NumberSymbols) -> Result<Self, NumberFmtError> {
        let mut has_exp = false;
        let mut has_exp_0 = false;
        let mut has_dec_sep = false;
        let mut has_frac_0 = false;
        let mut has_int_sign = false;
        let mut min_int_sign = 0;
        let mut has_exp_sign = false;
        let mut min_exp_sign = 0;
        let mut len_frac = 0;
        let mut len_int = 0;
        let mut len_exp = 0;

        let mut idx_frac = 0;
        for t in pattern.iter_mut() {
            match t {
                Token::DecimalSep | Token::DecimalSepAlways => {
                    if has_dec_sep {
                        return Err(NumberFmtError::ParseInvalidDecimalSep);
                    }
                    has_dec_sep = true;
                }
                Token::Digit0(Mode::Fraction, x) => {
                    has_frac_0 = true;
                    len_frac += 1;
                    *x = idx_frac;
                    idx_frac += 1;
                }
                Token::Digit(Mode::Fraction, x) => {
                    len_frac += 1;
                    *x = idx_frac;
                    idx_frac += 1;
                }
                Token::Numeric(Mode::Fraction, x, sign) => {
                    len_frac += 1;
                    *x = idx_frac;
                    *sign = false;
                    idx_frac += 1;
                }

                Token::ExponentLower | Token::ExponentUpper => {
                    if has_exp {
                        return Err(NumberFmtError::ParseInvalidExp);
                    }
                    has_exp = true;
                }

                Token::SignInt => {
                    if has_int_sign {
                        return Err(NumberFmtError::ParseInvalidSign);
                    }
                    has_int_sign = true;
                }
                Token::PlusInt => {
                    if has_int_sign {
                        return Err(NumberFmtError::ParseInvalidSign);
                    }
                    has_int_sign = true;
                }
                Token::SignExp => {
                    if has_exp_sign {
                        return Err(NumberFmtError::ParseInvalidExpSign);
                    }
                    has_exp_sign = true;
                }
                Token::PlusExp => {
                    if has_exp_sign {
                        return Err(NumberFmtError::ParseInvalidExpSign);
                    }
                    has_exp_sign = true;
                }

                _ => {}
            }
        }
        let mut idx_int = 0;
        let mut idx_exp = 0;
        let mut was_grp = false;
        for t in pattern.iter_mut().rev() {
            match t {
                Token::Digit0(Mode::Integer, x) => {
                    len_int += 1;
                    min_int_sign = idx_int + 1;
                    *x = idx_int;
                    idx_int += 1;
                }
                Token::Digit(Mode::Integer, x) => {
                    len_int += 1;
                    min_int_sign = idx_int + 1;
                    *x = idx_int;
                    idx_int += 1;
                }
                Token::Numeric(Mode::Integer, x, sign) => {
                    len_int += 1;
                    *x = idx_int;
                    *sign = !has_int_sign && (sym.decimal_grp.is_none() || !was_grp);
                    idx_int += 1;
                }

                Token::GroupingSep(x, sign) => {
                    *sign = !has_int_sign;
                    *x = idx_int;
                }

                Token::Digit0(Mode::Exponent, x) => {
                    len_exp += 1;
                    has_exp_0 = true;
                    min_exp_sign = idx_exp + 1;
                    *x = idx_exp;
                    idx_exp += 1;
                }
                Token::Digit(Mode::Exponent, x) => {
                    len_exp += 1;
                    min_exp_sign = idx_exp;
                    *x = idx_exp;
                    idx_exp += 1;
                }
                Token::Numeric(Mode::Exponent, x, sign) => {
                    len_exp += 1;
                    *x = idx_exp;
                    *sign = !has_exp_sign;
                    idx_exp += 1;
                }

                _ => {}
            }

            was_grp = matches!(t, Token::GroupingSep(_, _));
        }

        Ok(NumberFormat {
            min_int_sign,
            len_int,
            min_exp_sign,
            has_exp,
            len_exp,
            has_exp_0,
            has_frac_0,
            len_frac,
            tok: pattern,
            sym,
        })
    }

    /// Parses the format string. Uses the default symbol table.
    fn parse_tokens(pattern: &str) -> Result<Vec<Token>, NumberFmtError> {
        let mut esc = false;
        let mut mode = Mode::Integer;

        let mut tok = Vec::new();

        for m in pattern.chars() {
            let mask = if esc {
                esc = false;
                Token::Separator(m)
            } else {
                match m {
                    '0' => Token::Digit0(mode, 0),
                    '9' => Token::Digit(mode, 0),
                    '#' => Token::Numeric(mode, 0, false),
                    '.' => {
                        if matches!(mode, Mode::Fraction | Mode::Exponent) {
                            return Err(NumberFmtError::ParseInvalidDecimalSep);
                        }
                        mode = Mode::Fraction;
                        Token::DecimalSep
                    }
                    ':' => {
                        if matches!(mode, Mode::Fraction | Mode::Exponent) {
                            return Err(NumberFmtError::ParseInvalidDecimalSep);
                        }
                        mode = Mode::Fraction;
                        Token::DecimalSepAlways
                    }
                    ',' => Token::GroupingSep(0, false),
                    '-' => {
                        if mode == Mode::Integer {
                            Token::SignInt
                        } else if mode == Mode::Exponent {
                            Token::SignExp
                        } else {
                            return Err(NumberFmtError::ParseInvalidSign);
                        }
                    }
                    '+' => {
                        if mode == Mode::Integer {
                            Token::PlusInt
                        } else if mode == Mode::Exponent {
                            Token::PlusExp
                        } else {
                            return Err(NumberFmtError::ParseInvalidSign);
                        }
                    }
                    'e' => {
                        if mode == Mode::Exponent {
                            return Err(NumberFmtError::ParseInvalidExp);
                        }
                        mode = Mode::Exponent;
                        Token::ExponentLower
                    }
                    'E' => {
                        if mode == Mode::Exponent {
                            return Err(NumberFmtError::ParseInvalidExp);
                        }
                        mode = Mode::Exponent;
                        Token::ExponentUpper
                    }
                    '$' => Token::Currency,
                    '\\' => {
                        esc = true;
                        continue;
                    }
                    ' ' => Token::Separator(' '),
                    c if c.is_ascii() => return Err(NumberFmtError::ParseUnescaped),
                    c => Token::Separator(c),
                }
            };
            tok.push(mask);
        }

        Ok(tok)
    }

    /// Symbols
    pub fn sym(&self) -> &NumberSymbols {
        &self.sym
    }

    /// Formats and unwraps any error.
    /// The error is written to the result string using {:?}.
    /// So this one may be convenient in some situations, but ...
    #[inline]
    pub fn fmt_u<Number: LowerExp + Display>(&self, number: Number) -> String {
        let mut out = String::new();
        match core::format_to(number, self, self.sym(), &mut out) {
            Ok(_) => {}
            Err(e) => {
                out.clear();
                _ = write!(out, "{:?}", e);
            }
        }
        out
    }

    /// Formats.
    #[inline]
    pub fn fmt<Number: LowerExp + Display>(
        &self,
        number: Number,
    ) -> Result<String, NumberFmtError> {
        let mut out = String::new();
        core::format_to(number, self, self.sym(), &mut out)?;
        Ok(out)
    }

    /// Formats to a buffer.
    #[inline]
    pub fn fmt_to<Number: LowerExp + Display, W: FmtWrite>(
        &self,
        number: Number,
        out: &mut W,
    ) -> Result<(), NumberFmtError> {
        core::format_to(number, self, self.sym(), out)
    }

    /// Parse using the exact format.
    /// See [ParseNumber::parse_sym()](crate::number::ParseNumber::parse_sym()]
    #[inline]
    pub fn parse<F: FromStr>(&self, s: &str) -> Result<F, NumberFmtError> {
        core::parse_fmt(s, self, &self.sym)
    }
}

/// Parses a number from a &str.
pub trait ParseNumber {
    /// Parse the number after applying [core::clean_num()].
    /// This removes everything but digits, decimal sym and sign and then parses.
    /// Uses the given symbols for the translation.
    fn parse_sym<F: FromStr>(&self, sym: &NumberSymbols) -> Result<F, NumberFmtError>;
    /// Parse the number after applying [core::unmap_num()]
    /// Creates a raw number by unapplying the exact pattern.
    fn parse_fmt<F: FromStr>(&self, fmt: &NumberFormat) -> Result<F, NumberFmtError>;
}

impl ParseNumber for &str {
    fn parse_sym<F: FromStr>(&self, sym: &NumberSymbols) -> Result<F, NumberFmtError> {
        core::parse_sym(self, sym)
    }

    fn parse_fmt<F: FromStr>(&self, fmt: &NumberFormat) -> Result<F, NumberFmtError> {
        core::parse_fmt(self, fmt, &fmt.sym)
    }
}

/// Format a number according to a format string.
pub trait FormatNumber
where
    Self: Copy + LowerExp + Display,
{
    /// Format using the format-string. Uses the given symbols.
    fn format<'a>(
        &self,
        pattern: &'a str,
        sym: &'a NumberSymbols,
    ) -> Result<FormattedNumber<'a, Self>, NumberFmtError>;

    /// Format using the [NumberFormat]
    fn fmt<'a>(&self, format: &'a NumberFormat) -> RefFormattedNumber<'a, Self>;
}

/// Holds a temporary result from [FormatNumber]. The only purpose is as anchor for the
/// Display trait.
#[derive(Debug)]
pub struct FormattedNumber<'a, Number> {
    num: Number,
    format: NumberFormat,
    sym: &'a NumberSymbols,
}

impl<'a, Number: Copy + LowerExp + Display> Display for FormattedNumber<'a, Number> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match core::format_to(self.num, &self.format, self.sym, f) {
            Ok(_) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}

/// Holds a temporary result from [FormatNumber]. The only purpose is as anchor for the
/// Display trait.
#[derive(Debug)]
pub struct RefFormattedNumber<'a, Number> {
    num: Number,
    format: &'a NumberFormat,
}

impl<'a, Number: Copy + LowerExp + Display> Display for RefFormattedNumber<'a, Number> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match core::format_to(self.num, self.format, &self.format.sym, f) {
            Ok(_) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}

macro_rules! define_fmt {
    ($t:ty) => {
        impl FormatNumber for $t {
            #[inline]
            fn format<'a>(
                &self,
                pattern: &'a str,
                sym: &'a NumberSymbols,
            ) -> Result<FormattedNumber<'a, Self>, NumberFmtError> {
                Ok(FormattedNumber {
                    num: *self,
                    format: NumberFormat::new(pattern)?,
                    sym,
                })
            }

            #[inline]
            fn fmt<'a>(&self, format: &'a NumberFormat) -> RefFormattedNumber<'a, Self> {
                RefFormattedNumber { num: *self, format }
            }
        }
    };
}

define_fmt!(f64);
define_fmt!(f32);
define_fmt!(u128);
define_fmt!(u64);
define_fmt!(u32);
define_fmt!(u16);
define_fmt!(u8);
define_fmt!(i128);
define_fmt!(i64);
define_fmt!(i32);
define_fmt!(i16);
define_fmt!(i8);
define_fmt!(usize);
define_fmt!(isize);
define_fmt!(Decimal);

pub mod core {
    use crate::{Mode, NumberFmtError, NumberFormat, NumberSymbols, Token};
    #[allow(unused_imports)]
    use log::debug;
    use memchr::memchr;
    use std::cell::Cell;
    use std::cmp::max;
    use std::fmt::{Display, LowerExp, Write as FmtWrite};
    use std::str::FromStr;

    fn split_num(value: &str) -> (&str, &str, &str, &str, &str) {
        // everything is ascii
        let bytes = value.as_bytes();
        let len = bytes.len();

        let idx_sep = memchr(b'.', bytes);
        let idx_exp = memchr(b'e', bytes);

        let digits_end = if let Some(idx_sep) = idx_sep {
            idx_sep
        } else if let Some(idx_exp) = idx_exp {
            idx_exp
        } else {
            len
        };

        let fraction_end = if let Some(idx_exp) = idx_exp {
            idx_exp
        } else {
            len
        };

        let (r_sign, r_digits) = if len > 0 && bytes[0] == b'-' {
            (0usize..1usize, 1usize..digits_end)
        } else {
            (0usize..0usize, 0usize..digits_end)
        };
        let r_fraction = if let Some(idx_sep) = idx_sep {
            idx_sep + 1..fraction_end
        } else {
            fraction_end..fraction_end
        };
        let (r_sign_exp, r_exp) = if let Some(idx_exp) = idx_exp {
            if idx_exp + 1 < len && bytes[idx_exp + 1] == b'-' {
                (idx_exp + 1..idx_exp + 2, idx_exp + 2..len)
            } else {
                (idx_exp + 1..idx_exp + 1, idx_exp + 1..len)
            }
        } else {
            (len..len, len..len)
        };

        (
            &value[r_sign],
            &value[r_digits],
            &value[r_fraction],
            &value[r_sign_exp],
            &value[r_exp],
        )
    }

    /// Get the clean number.
    ///
    /// Takes only digits and maps backwards according to the symbol table.
    /// This will only work if you don't use separators that can be mistaken
    /// with one of those symbols.
    ///
    /// Removes any leading zeros too.
    pub fn clean_num<W: FmtWrite>(
        formatted: &str,
        sym: &NumberSymbols,
        out: &mut W,
    ) -> Result<(), NumberFmtError> {
        let mut seen_non_0 = false;
        for c in formatted.chars() {
            if c.is_ascii_digit() {
                seen_non_0 |= c != '0';
                if seen_non_0 {
                    out.write_char(c)?;
                }
            } else if c == sym.negative_sym {
                out.write_char('-')?;
            } else if c == sym.positive_sym {
                // noop
            } else if c == '+' {
                // todo: ???
            } else if c == sym.decimal_sep {
                out.write_char('.')?;
            } else if c == sym.exponent_lower_sym || c == sym.exponent_upper_sym {
                out.write_char('e')?;
            }
        }
        Ok(())
    }

    /// Unmap the formatted string back to a format that `f64::parse()` can understand.
    #[allow(clippy::if_same_then_else)]
    pub fn unmap_num<W: FmtWrite>(
        formatted: &str,
        format: &NumberFormat,
        sym: &NumberSymbols,
        out: &mut W,
    ) -> Result<(), NumberFmtError> {
        let mut buf_sign = String::new();
        let mut buf_int = String::new();
        let mut buf_frac = String::new();
        let mut buf_exp_sign = String::new();
        let mut buf_exp = String::new();

        let mut it = format.tok.iter();
        let mut jt = formatted.chars();
        loop {
            let Some(t) = it.next() else {
                break;
            };
            let Some(c) = jt.next() else {
                break;
            };

            match t {
                Token::SignInt => {
                    if c == sym.negative_sym {
                        buf_sign.push('-');
                    } else if c == sym.positive_sym {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidSign);
                    }
                }
                Token::PlusInt => {
                    if c == '-' {
                        buf_sign.push('-');
                    } else if c == '+' {
                        buf_sign.push('+');
                    } else {
                        return Err(NumberFmtError::ParseInvalidSign);
                    }
                }
                Token::Digit0(Mode::Integer, _) => {
                    if c.is_ascii_digit() {
                        buf_int.push(c);
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Digit(Mode::Integer, _) => {
                    if c.is_ascii_digit() {
                        buf_int.push(c);
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Numeric(Mode::Integer, _, _) => {
                    if c.is_ascii_digit() {
                        buf_int.push(c);
                    } else if c == sym.negative_sym {
                        buf_sign.push('-');
                    } else if c == sym.positive_sym || c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::GroupingSep(_, _) => {
                    if let Some(decimal_grp) = sym.decimal_grp {
                        if c == decimal_grp {
                            // ok
                        } else if c == sym.negative_sym {
                            buf_sign.push('-');
                        } else if c == sym.positive_sym || c == ' ' {
                            // ok
                        } else {
                            return Err(NumberFmtError::ParseInvalidGroupingSep);
                        }
                    }
                }
                Token::DecimalSep => {
                    if c == sym.decimal_sep {
                        buf_frac.push('.');
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDecimalSep);
                    }
                }
                Token::DecimalSepAlways => {
                    if c == sym.decimal_sep {
                        buf_frac.push('.');
                    } else {
                        return Err(NumberFmtError::ParseInvalidDecimalSep);
                    }
                }
                Token::Digit0(Mode::Fraction, _) => {
                    if c.is_ascii_digit() {
                        buf_frac.push(c);
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Digit(Mode::Fraction, _) => {
                    if c.is_ascii_digit() {
                        buf_frac.push(c);
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Numeric(Mode::Fraction, _, _) => {
                    if c.is_ascii_digit() {
                        buf_frac.push(c);
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::ExponentUpper => {
                    if c == sym.exponent_upper_sym {
                        // ok
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidExp);
                    }
                }
                Token::ExponentLower => {
                    if c == sym.exponent_lower_sym {
                        // ok
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidExp);
                    }
                }
                Token::Digit0(Mode::Exponent, _) => {
                    if c.is_ascii_digit() {
                        buf_exp.push(c);
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Digit(Mode::Exponent, _) => {
                    if c.is_ascii_digit() {
                        buf_exp.push(c);
                    } else if c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::Numeric(Mode::Exponent, _, _) => {
                    if c.is_ascii_digit() {
                        buf_exp.push(c);
                    } else if c == sym.negative_sym {
                        buf_exp_sign.push('-');
                    } else if c == sym.positive_sym || c == ' ' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidDigit);
                    }
                }
                Token::SignExp => {
                    if c == sym.negative_sym {
                        buf_exp_sign.push('-');
                    } else if c == sym.positive_sym || c == '+' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidExpSign);
                    }
                }
                Token::PlusExp => {
                    if c == '-' {
                        buf_exp_sign.push('-');
                    } else if c == '+' {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidExpSign);
                    }
                }

                Token::Currency => {
                    let mut kt = sym.currency_sym.as_str().chars();
                    let s = kt.next();
                    if Some(c) != s {
                        return Err(NumberFmtError::ParseInvalidCurrency);
                    }

                    loop {
                        match kt.next() {
                            None => {
                                break;
                            }
                            Some(s) => {
                                let Some(c) = jt.next() else {
                                    return Err(NumberFmtError::ParseInvalidCurrency);
                                };
                                if c != s {
                                    return Err(NumberFmtError::ParseInvalidCurrency);
                                }
                            }
                        }
                    }
                }

                Token::Separator(sep) => {
                    if c == *sep {
                        // ok
                    } else {
                        return Err(NumberFmtError::ParseInvalidSeparator);
                    }
                }
            }
        }

        out.write_str(buf_sign.as_str())?;
        out.write_str(buf_int.as_str())?;
        out.write_str(buf_frac.as_str())?;
        if !buf_exp.is_empty() {
            out.write_char('e')?;
        }
        out.write_str(buf_exp_sign.as_str())?;
        out.write_str(buf_exp.as_str())?;

        Ok(())
    }

    /// Takes a raw number string and applies the format.
    ///
    /// The raw number should be in a format produced by the format! macro. decimal point is '.',
    /// exponent is 'e' and negative sign is '-'.
    #[inline]
    pub fn map_num<W: FmtWrite, const EXP: bool>(
        raw: &str,
        format: &NumberFormat,
        sym: &NumberSymbols,
        out: &mut W,
    ) -> Result<(), NumberFmtError> {
        let (raw_sign, raw_int, raw_frac, raw_exp_sign, raw_exp) = split_num(raw);

        // locale mapping

        // grouping
        let skip_group = sym.decimal_grp.is_none();
        let disp_decimal_grp = if let Some(decimal_grp) = sym.decimal_grp {
            decimal_grp
        } else {
            ' '
        };

        // sign
        let disp_sign = if raw_sign.is_empty() {
            sym.positive_sym
        } else {
            sym.negative_sym
        };

        // integer
        let int = raw_int.as_bytes();
        let len_int = int.len() as u32;
        if len_int > format.len_int {
            return Err(NumberFmtError::FmtLenInt);
        }

        // dec-sep
        let disp_decimal_sep = if !raw_frac.is_empty() || format.has_frac_0 {
            sym.decimal_sep
        } else {
            ' '
        };

        // fraction
        let frac = raw_frac.as_bytes();
        let len_frac = frac.len() as u32;

        // exponent sign
        let len_exp_sign = raw_exp_sign.len() as u32;

        // exponent
        let exp = raw_exp.as_bytes();
        let len_exp = exp.len() as u32;

        let (disp_exp_upper, disp_exp_lower, disp_exp_sign, shift_exp_n, shift_exp_pos) = if EXP {
            let disp_exp_upper = if !raw_exp.is_empty() || format.has_exp_0 {
                sym.exponent_upper_sym
            } else {
                ' '
            };
            let disp_exp_lower = if !raw_exp.is_empty() || format.has_exp_0 {
                sym.exponent_lower_sym
            } else {
                ' '
            };
            let disp_exp_sign = if raw_exp_sign.is_empty() {
                sym.positive_sym
            } else {
                sym.negative_sym
            };

            if len_exp > format.len_exp {
                return Err(NumberFmtError::FmtLenExp);
            }
            // not enough space for the exponent
            if max(len_exp, format.min_exp_sign) + len_exp_sign > format.len_exp {
                return Err(NumberFmtError::FmtLenExp);
            }
            // left shift the exponent and fill the rest with ' '.
            let shift_exp_n = format.len_exp - max(len_exp, format.min_exp_sign) - len_exp_sign;
            let shift_exp_pos = max(len_exp, format.min_exp_sign) + len_exp_sign;

            (
                disp_exp_upper,
                disp_exp_lower,
                disp_exp_sign,
                shift_exp_n,
                shift_exp_pos,
            )
        } else {
            (' ', ' ', ' ', 0, 0)
        };

        let mut used_sign = false;
        let mut used_exp_sign = false;

        for m in format.tok.iter() {
            match m {
                Token::SignInt => {
                    debug_assert!(!used_sign);
                    out.write_char(disp_sign)?;
                    used_sign = true;
                }
                Token::PlusInt => {
                    debug_assert!(!used_sign);
                    if raw_sign.is_empty() {
                        out.write_char('+')?;
                    } else {
                        out.write_char('-')?;
                    }
                }
                Token::GroupingSep(i, can_be_sign) => {
                    if skip_group {
                        // noop
                    } else if len_int > *i {
                        out.write_char(disp_decimal_grp)?;
                    } else if *can_be_sign && max(len_int, format.min_int_sign) == *i {
                        debug_assert!(!used_sign);
                        out.write_char(disp_sign)?;
                        used_sign = true;
                    } else {
                        out.write_char(' ')?;
                    }
                }
                Token::Digit0(Mode::Integer, i) => {
                    if len_int > *i {
                        out.write_char(int[(len_int - i - 1) as usize] as char)?;
                    } else {
                        out.write_char('0')?;
                    }
                }
                Token::Digit(Mode::Integer, i) => {
                    if len_int > *i {
                        out.write_char(int[(len_int - i - 1) as usize] as char)?;
                    } else {
                        out.write_char(' ')?;
                    }
                }
                Token::Numeric(Mode::Integer, i, can_be_sign) => {
                    if len_int > *i {
                        out.write_char(int[(len_int - i - 1) as usize] as char)?;
                    } else if *can_be_sign && max(len_int, format.min_int_sign) == *i {
                        debug_assert!(!used_sign);
                        out.write_char(disp_sign)?;
                        used_sign = true;
                    } else {
                        out.write_char(' ')?;
                    }
                }
                Token::DecimalSep => {
                    out.write_char(disp_decimal_sep)?;
                }
                Token::DecimalSepAlways => {
                    out.write_char(sym.decimal_sep)?;
                }
                Token::Digit0(Mode::Fraction, i) => {
                    if len_frac > *i {
                        out.write_char(frac[*i as usize] as char)?;
                    } else {
                        out.write_char('0')?;
                    }
                }
                Token::Digit(Mode::Fraction, i) => {
                    if len_frac > *i {
                        out.write_char(frac[*i as usize] as char)?;
                    } else {
                        out.write_char(' ')?;
                    }
                }
                Token::Numeric(Mode::Fraction, i, _) => {
                    if len_frac > *i {
                        out.write_char(frac[*i as usize] as char)?;
                    } else {
                        out.write_char(' ')?;
                    }
                }
                Token::ExponentUpper => {
                    if EXP {
                        out.write_char(disp_exp_upper)?;
                    }
                }
                Token::ExponentLower => {
                    if EXP {
                        out.write_char(disp_exp_lower)?;
                    }
                }
                Token::SignExp => {
                    if EXP {
                        debug_assert!(!used_exp_sign);
                        if raw_exp_sign.is_empty() && sym.positive_sym == ' ' {
                            // explicit sign in the exponent shows '+'.
                            out.write_char('+')?;
                        } else {
                            out.write_char(disp_exp_sign)?;
                        }
                        used_exp_sign = true;
                    }
                }
                Token::PlusExp => {
                    if EXP {
                        debug_assert!(!used_exp_sign);
                        if raw_exp_sign.is_empty() {
                            out.write_char('+')?;
                        } else {
                            out.write_char('-')?;
                        }
                        used_exp_sign = true;
                    }
                }
                Token::Digit0(Mode::Exponent, i) => {
                    if EXP {
                        if *i >= shift_exp_pos {
                            // left-shift exponent
                        } else if len_exp > *i {
                            out.write_char(exp[(len_exp - i - 1) as usize] as char)?;
                        } else {
                            out.write_char('0')?;
                        }
                        // append shifted digits as blank
                        if *i == 0 {
                            for _ in 0..shift_exp_n {
                                out.write_char(' ')?;
                            }
                        }
                    }
                }
                Token::Digit(Mode::Exponent, i) => {
                    if EXP {
                        if *i >= shift_exp_pos {
                            // left-shift exponent
                        } else if len_exp > *i {
                            out.write_char(exp[(len_exp - i - 1) as usize] as char)?;
                        } else {
                            out.write_char(' ')?;
                        }
                        // append shifted digits as blank
                        if *i == 0 {
                            for _ in 0..shift_exp_n {
                                out.write_char(' ')?;
                            }
                        }
                    }
                }
                Token::Numeric(Mode::Exponent, i, can_be_sign) => {
                    if EXP {
                        if *i >= shift_exp_pos {
                            // left-shift exponent
                        } else if len_exp > *i {
                            out.write_char(exp[(len_exp - i - 1) as usize] as char)?;
                        } else if *can_be_sign && max(len_exp, format.min_exp_sign) == *i {
                            debug_assert!(!used_exp_sign);
                            out.write_char(disp_exp_sign)?;
                            used_exp_sign = true;
                        } else {
                            out.write_char(' ')?;
                        }

                        // append shifted digits as blank
                        if *i == 0 {
                            for _ in 0..shift_exp_n {
                                out.write_char(' ')?;
                            }
                        }
                    }
                }
                Token::Currency => {
                    out.write_str(sym.currency_sym.as_str())?;
                }
                Token::Separator(v) => {
                    out.write_char(*v)?;
                }
            }
        }

        if !used_sign && !raw_sign.is_empty() {
            return Err(NumberFmtError::FmtNoSign);
        }
        if !used_exp_sign && !raw_exp_sign.is_empty() {
            return Err(NumberFmtError::FmtNoExpSign);
        }

        Ok(())
    }

    /// Formats the number and writes the result to out.
    pub fn format_to<W: FmtWrite, Number: LowerExp + Display>(
        number: Number,
        format: &NumberFormat,
        sym: &NumberSymbols,
        out: &mut W,
    ) -> Result<(), NumberFmtError> {
        thread_local! {
            static RAW: Cell<String> = const {Cell::new(String::new())};
        }

        let mut raw = RAW.take();

        raw.clear();
        let res = if format.has_exp {
            write!(raw, "{:.*e}", format.len_frac as usize, number)
                .map_err(|_| NumberFmtError::Fmt)?;
            map_num::<_, true>(raw.as_str(), format, sym, out)
        } else {
            write!(raw, "{:.*}", format.len_frac as usize, number)
                .map_err(|_| NumberFmtError::Fmt)?;
            map_num::<_, false>(raw.as_str(), format, sym, out)
        };

        match res {
            Ok(v) => {
                RAW.set(raw);
                Ok(v)
            }
            Err(e) => {
                RAW.set(raw);
                Err(e)
            }
        }
    }

    /// Parse the number according to the exact format.
    pub fn parse_fmt<F: FromStr>(
        s: &str,
        fmt: &NumberFormat,
        sym: &NumberSymbols,
    ) -> Result<F, NumberFmtError> {
        thread_local! {
            static RAW: Cell<String> = const {Cell::new(String::new())};
        }

        let mut raw = RAW.take();

        raw.clear();
        unmap_num(s, fmt, sym, &mut raw)?;

        match raw.parse::<F>() {
            Ok(v) => {
                RAW.set(raw);
                Ok(v)
            }
            Err(_) => {
                RAW.set(raw);
                Err(NumberFmtError::Parse)
            }
        }
    }

    /// Parse the number only using the symbols for translation.
    /// Takes digits and some specials and ignores the rest.
    pub fn parse_sym<F: FromStr>(s: &str, sym: &NumberSymbols) -> Result<F, NumberFmtError> {
        thread_local! {
            static RAW: Cell<String> = const {Cell::new(String::new())};
        }

        let mut raw = RAW.take();

        raw.clear();
        clean_num(s, sym, &mut raw)?;

        match raw.parse::<F>() {
            Ok(v) => {
                RAW.set(raw);
                Ok(v)
            }
            Err(_) => {
                RAW.set(raw);
                Err(NumberFmtError::Parse)
            }
        }
    }
}

/// Format a Number according to the format string.
/// Uses the default symbols.
pub fn format<Number: LowerExp + Display>(
    number: Number,
    pattern: &str,
) -> Result<String, NumberFmtError> {
    let fmt = NumberFormat::new(pattern)?;
    let mut out = String::new();
    core::format_to(number, &fmt, fmt.sym(), &mut out)?;
    Ok(out)
}

/// Format a Number according to the format string.
/// Uses the default symbols.
pub fn format_to<W: FmtWrite, Number: LowerExp + Display>(
    number: Number,
    pattern: &str,
    out: &mut W,
) -> Result<(), NumberFmtError> {
    let fmt = NumberFormat::new(pattern)?;
    core::format_to(number, &fmt, fmt.sym(), out)
}

/// Format a Number according to the format string.
pub fn formats<Number: LowerExp + Display>(
    number: Number,
    pattern: &str,
    sym: &NumberSymbols,
) -> Result<String, NumberFmtError> {
    let format = NumberFormat::new(pattern)?;
    let mut out = String::new();
    core::format_to(number, &format, sym, &mut out)?;
    Ok(out)
}

/// Format a Number according to the format string.
pub fn formats_to<W: FmtWrite, Number: LowerExp + Display>(
    number: Number,
    pattern: &str,
    sym: &NumberSymbols,
    out: &mut W,
) -> Result<(), NumberFmtError> {
    let format = NumberFormat::new(pattern)?;
    core::format_to(number, &format, sym, out)
}

/// Format a Number according to the format.
pub fn fmt<Number: LowerExp + Display>(number: Number, format: &NumberFormat) -> String {
    let mut out = String::new();
    _ = core::format_to(number, format, &format.sym, &mut out);
    out
}

/// Format a Number according to the format.
pub fn fmt_to<W: FmtWrite, Number: LowerExp + Display>(
    number: Number,
    format: &NumberFormat,
    out: &mut W,
) {
    _ = core::format_to(number, format, &format.sym, out)
}

/// Parse using the NumberSymbols.
/// Parses the number after applying [core::clean_num]
pub fn parse_sym<F: FromStr>(s: &str, sym: &NumberSymbols) -> Result<F, NumberFmtError> {
    core::parse_sym(s, sym)
}

/// Parse using the NumberFormat.
/// Parses the number after applying [core::unmap_num]
pub fn parse_fmt<F: FromStr>(s: &str, fmt: &NumberFormat) -> Result<F, NumberFmtError> {
    core::parse_fmt(s, fmt, &fmt.sym)
}

/// Parse using the NumberFormat.
/// Parses the number after applying [core::unmap_num]
pub fn parse_format<F: FromStr>(
    s: &str,
    pattern: &str,
    sym: &NumberSymbols,
) -> Result<F, NumberFmtError> {
    let format = NumberFormat::new(pattern)?;
    core::parse_fmt(s, &format, sym)
}
