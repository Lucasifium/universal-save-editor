//! Reading what is typed into a number box in the interface language's way of writing numbers,
//! digit grouping for showing them, and quick maths (`+500`, `*2`, `max`, `250k`) worked out when
//! the typing is done.
//!
//! Reading is strict: "1.500" is fifteen hundred in German and one and a half in English, so a
//! text is only taken when it can mean one thing in the interface language. Anything else (a
//! group that isn't three digits, separators in the wrong order, a decimal part in a whole-number
//! field) is refused and the box turns red, rather than guessed.

use serde_json::{Number, Value};

/// How the interface language writes numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumLocale {
    /// Between groups of three digits (also accepted: spaces, no-break and narrow no-break
    /// spaces, and "_", in every language).
    pub group: char,
    /// Before the decimal part.
    pub decimal: char,
}

/// "1,234,567.5": English, Japanese, Korean, Chinese.
pub const POINT: NumLocale = NumLocale { group: ',', decimal: '.' };
/// "1.234.567,5": German, Spanish, Portuguese, Italian, Turkish.
pub const COMMA: NumLocale = NumLocale { group: '.', decimal: ',' };
/// "1 234 567,5": French, Polish, Russian (a no-break space, so a number never splits over two lines).
pub const SPACE: NumLocale = NumLocale { group: '\u{00A0}', decimal: ',' };

/// The number style of a language code.
pub fn locale_for(code: &str) -> NumLocale {
    match code {
        "de" | "es" | "pt-BR" | "it" | "tr" => COMMA,
        "fr" | "pl" | "ru" => SPACE,
        _ => POINT,
    }
}

/// The number style of the interface language.
pub fn locale() -> NumLocale {
    locale_for(crate::i18n::current().code)
}

/// The digit-group separator of the interface language.
pub fn group_separator() -> char {
    locale().group
}

/// Whole numbers from this size up are shown grouped; smaller ones (a year, a level) stay plain.
const GROUP_FROM: u64 = 10_000;

/// `plain` (the text of a whole number, such as "-3520485254") with its digits in groups of
/// three. Anything that isn't a plain whole number, or is under 10,000, comes back as it is.
pub fn group_digits(plain: &str, sep: char) -> String {
    let (sign, digits) = match plain.strip_prefix('-') {
        Some(d) => ("-", d),
        None => ("", plain),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || digits.parse::<u64>().is_ok_and(|n| n < GROUP_FROM) {
        return plain.to_string();
    }
    let mut out = String::with_capacity(plain.len() + digits.len() / 3 * 2);
    out.push_str(sign);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

/// A number's text for typing into, in `loc`'s style: plain digits, and the language's decimal
/// mark ("2,5" in German), so that what the box starts with reads back as the same number.
pub fn edit_text(n: &Number, loc: NumLocale) -> String {
    let t = crate::widgets::number_text(n);
    if n.is_i64() || n.is_u64() || loc.decimal == '.' {
        t
    } else {
        t.replace('.', &loc.decimal.to_string())
    }
}

/// Spaces that may sit between digit groups in any language.
fn is_space_group(c: char) -> bool {
    matches!(c, ' ' | '\u{00A0}' | '\u{202F}' | '_')
}

/// `text` as a plain number ("-1234.5": an optional minus, digits, and "." before a decimal
/// part), read in `loc`'s style. `None` when it isn't a well-formed number there, or has a
/// decimal part where `integer` asks for a whole number.
pub fn normalize(text: &str, loc: NumLocale, integer: bool) -> Option<String> {
    let t = text.trim();
    let (neg, t) = match t.strip_prefix(['-', '\u{2212}']) {
        Some(r) => (true, r),
        None => (false, t),
    };
    let mut parts = t.split(loc.decimal);
    let int = parts.next()?;
    let frac = parts.next();
    if parts.next().is_some() || (integer && frac.is_some()) {
        return None;
    }
    let is_group = |c: char| c == loc.group || is_space_group(c);
    let mut digits = String::new();
    if int.chars().any(is_group) {
        // Grouped: 1 to 3 digits first, then groups of exactly three.
        for (i, g) in int.split(is_group).enumerate() {
            let ok = !g.is_empty() && g.bytes().all(|b| b.is_ascii_digit()) && if i == 0 { g.len() <= 3 } else { g.len() == 3 };
            if !ok {
                return None;
            }
            digits.push_str(g);
        }
    } else if int.bytes().all(|b| b.is_ascii_digit()) {
        digits.push_str(int);
    } else {
        return None;
    }
    let frac = match frac {
        Some(f) if !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()) => Some(f),
        Some(_) => return None,
        None => None,
    };
    if digits.is_empty() && frac.is_none() {
        return None;
    }
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(if digits.is_empty() { "0" } else { &digits });
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    Some(out)
}

/// A typed number read in `loc`'s style: whole numbers stay whole (`integer`), decimals become
/// floats. `None` when it can't be read without guessing.
pub fn parse_in(text: &str, integer: bool, loc: NumLocale) -> Option<Value> {
    let t = normalize(text, loc, integer)?;
    if integer {
        if let Ok(i) = t.parse::<i64>() {
            return Some(Value::from(i));
        }
        return t.parse::<u64>().ok().map(Value::from);
    }
    let f: f64 = t.parse().ok()?;
    Number::from_f64(f).map(Value::Number)
}

/// Whether typed text is quick maths (worked out when the box is left or Enter is pressed)
/// rather than a plain number (taken while typing). A leading "-" is always a plain negative
/// number, never "subtract": typing -5 into a signed field must give -5.
pub fn is_quick_maths(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    if matches!(t.as_str(), "max" | "min") {
        return true;
    }
    t.starts_with(['+', '*', 'x', '\u{00D7}', '/']) || t.ends_with(['k', 'm'])
}

/// An amount typed in quick maths: whole (kept exact) or with a fraction.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Amount {
    Whole(i128),
    Frac(f64),
}

impl Amount {
    fn f64(self) -> f64 {
        match self {
            Amount::Whole(i) => i as f64,
            Amount::Frac(f) => f,
        }
    }
}

/// "250k", "1,5m" (German), "12": the amount, with k meaning thousands and m millions.
fn amount(t: &str, loc: NumLocale) -> Option<Amount> {
    let t = t.trim();
    let (num, mul) = if let Some(n) = t.strip_suffix('k') {
        (n, 1_000)
    } else if let Some(n) = t.strip_suffix('m') {
        (n, 1_000_000)
    } else {
        (t, 1)
    };
    let plain = normalize(num, loc, false)?;
    match plain.split_once('.') {
        None => plain.parse::<i128>().ok()?.checked_mul(mul).map(Amount::Whole),
        Some((w, f)) => {
            // "1.5k": exact when the suffix leaves no fraction.
            let scale = 10i128.checked_pow(f.len() as u32)?;
            let whole = format!("{w}{f}").parse::<i128>().ok()?.checked_mul(mul)?;
            if whole % scale == 0 {
                Some(Amount::Whole(whole / scale))
            } else {
                let x = plain.parse::<f64>().ok()? * mul as f64;
                x.is_finite().then_some(Amount::Frac(x))
            }
        }
    }
}

/// `quick_maths_in` in the interface language's number style.
pub fn quick_maths(text: &str, current: &Value, integer: bool, range: Option<(Value, Value)>) -> Option<Value> {
    quick_maths_in(text, current, integer, range, locale())
}

/// Works out quick maths typed into a number box holding `current`:
/// `+N` adds, `*N` (or `xN`) multiplies, `/N` divides, `max` and `min` take the ends of
/// `range` (the field's real limit where the format knows one, otherwise what its storage type
/// holds), and a `k` or `m` after a number means thousands or millions (`250k`, `-2k`,
/// `+1.5m`). Numbers are read in `loc`'s style. Whole numbers are worked out exactly; a
/// whole-number field gets the nearest whole number after `/` or a fractional factor, but a
/// fractional amount to add or set is refused. The result is pulled into `range`. `None`: not
/// quick maths, or nothing sensible came out (dividing by zero, `max` without a known range).
pub fn quick_maths_in(text: &str, current: &Value, integer: bool, range: Option<(Value, Value)>, loc: NumLocale) -> Option<Value> {
    let t = text.trim().to_lowercase();
    if !is_quick_maths(&t) {
        return None;
    }
    match t.as_str() {
        "max" => return range.map(|r| crate::widgets::same_kind(current, r.1)),
        "min" => return range.map(|r| crate::widgets::same_kind(current, r.0)),
        _ => {}
    }
    let cur_whole = current.as_i64().map(i128::from).or_else(|| current.as_u64().map(i128::from)).filter(|_| integer);
    let cur = || current.as_f64();
    let result: Amount = if let Some(rest) = t.strip_prefix('+') {
        match (amount(rest.trim_start(), loc)?, cur_whole) {
            (Amount::Whole(n), Some(c)) => Amount::Whole(c.checked_add(n)?),
            (Amount::Frac(_), Some(_)) => return None,
            (n, None) => Amount::Frac(cur()? + n.f64()),
        }
    } else if let Some(rest) = t.strip_prefix(['*', 'x', '\u{00D7}']) {
        match (amount(rest.trim_start(), loc)?, cur_whole) {
            (Amount::Whole(n), Some(c)) => Amount::Whole(c.checked_mul(n)?),
            (n, _) => Amount::Frac(cur()? * n.f64()),
        }
    } else if let Some(rest) = t.strip_prefix('/') {
        match (amount(rest.trim_start(), loc)?, cur_whole) {
            (Amount::Whole(0), _) => return None,
            (Amount::Whole(n), Some(c)) => Amount::Whole(div_round(c, n)),
            (n, _) if n.f64() == 0.0 => return None,
            (n, _) => Amount::Frac(cur()? / n.f64()),
        }
    } else {
        match amount(&t, loc)? {
            Amount::Frac(_) if integer => return None,
            a => a,
        }
    };
    to_value(result, integer, current, range)
}

/// `a / b` rounded to the nearest whole number (halves away from zero).
fn div_round(a: i128, b: i128) -> i128 {
    let (q, r) = (a / b, a % b);
    // Unsigned: the magnitude of the smallest i128 (a divisor typed in full) has no i128.
    if 2 * r.unsigned_abs() >= b.unsigned_abs() {
        q + if (a < 0) == (b < 0) { 1 } else { -1 }
    } else {
        q
    }
}

fn to_value(x: Amount, integer: bool, current: &Value, range: Option<(Value, Value)>) -> Option<Value> {
    let v = if integer {
        let w = match x {
            Amount::Whole(i) => i,
            Amount::Frac(f) if f.is_finite() && f.abs() < 1e30 => f.round() as i128,
            Amount::Frac(_) => return None,
        };
        if let Ok(i) = i64::try_from(w) {
            Value::from(i)
        } else if let Ok(u) = u64::try_from(w) {
            Value::from(u)
        } else if w < 0 {
            Value::from(i64::MIN)
        } else {
            Value::from(u64::MAX)
        }
    } else {
        let f = x.f64();
        if !f.is_finite() {
            return None;
        }
        Value::Number(Number::from_f64(f)?)
    };
    Some(crate::widgets::same_kind(current, crate::widgets::clamp_to(range, v)))
}

/// Whether `lo..=hi` is only the range of the type the value is stored as (a 32-bit whole
/// number: -2147483648 to 2147483647), rather than a limit the game or format really sets.
/// A storage type's range says nothing useful to the player, so it is not shown.
pub fn is_type_range(lo: &Value, hi: &Value) -> bool {
    if let (Some(l), Some(h)) = (lo.as_i64().map(i128::from).or_else(|| lo.as_u64().map(i128::from)), hi.as_i64().map(i128::from).or_else(|| hi.as_u64().map(i128::from))) {
        for bits in [8u32, 16, 32, 64] {
            let umax = (1i128 << bits) - 1;
            let smax = (1i128 << (bits - 1)) - 1;
            if (l == 0 && h == umax) || (l == -smax - 1 && h == smax) || (l == 0 && h == smax) {
                return true;
            }
        }
        return false;
    }
    // Floats: the whole range of a 32- or 64-bit float.
    match (lo.as_f64(), hi.as_f64()) {
        (Some(l), Some(h)) => h >= f32::MAX as f64 * 0.999 && (l <= -(f32::MAX as f64) * 0.999 || l == 0.0),
        _ => false,
    }
}

/// Whether `lo..=hi` is what a field of some number of bits holds (0 to 1023 for 10 bits,
/// 0 to 33,554,431 for 25): a packed field's width, which may be far above what the game
/// allows (Diablo II keeps gold in 25 bits but caps it at 10,000 per level). Such a range is
/// still a limit of the save, but it is not shown as the field's cap beside the box.
pub fn is_bit_width(lo: &Value, hi: &Value) -> bool {
    let whole = |v: &Value| v.as_i64().map(i128::from).or_else(|| v.as_u64().map(i128::from));
    let (Some(l), Some(h)) = (whole(lo), whole(hi)) else { return false };
    let pow2 = |x: i128| x > 0 && x & (x - 1) == 0;
    (l == 0 && h >= 255 && pow2(h + 1)) || (l < 0 && h > 0 && -l == h + 1 && pow2(h + 1) && h >= 127)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The number style of each language the tables below are written for.
    fn loc(lang: &str) -> NumLocale {
        locale_for(lang)
    }

    #[test]
    fn languages_have_their_number_style() {
        assert_eq!(locale_for("en"), POINT);
        assert_eq!(locale_for("ja"), POINT);
        assert_eq!(locale_for("zh-CN"), POINT);
        assert_eq!(locale_for("de"), COMMA);
        assert_eq!(locale_for("es"), COMMA);
        assert_eq!(locale_for("pt-BR"), COMMA);
        assert_eq!(locale_for("fr"), SPACE);
        assert_eq!(locale_for("it"), COMMA);
        assert_eq!(locale_for("tr"), COMMA);
        assert_eq!(locale_for("pl"), SPACE);
        assert_eq!(locale_for("ru"), SPACE);
        assert_eq!(locale_for("ko"), POINT);
        assert_eq!(locale_for("zh-TW"), POINT);
    }

    #[test]
    fn plain_numbers_are_read_in_the_language_style() {
        // (language, typed, whole-number field, read as)
        let cases: &[(&str, &str, bool, Option<Value>)] = &[
            ("en", "1500", true, Some(json!(1500))),
            ("en", "1,500", true, Some(json!(1500))),
            ("en", "1,234,567", true, Some(json!(1_234_567))),
            ("en", "1 500", true, Some(json!(1500))),
            ("en", "1_500", true, Some(json!(1500))),
            ("en", " -7 ", true, Some(json!(-7))),
            ("en", "\u{2212}7", true, Some(json!(-7))),
            ("en", "18446744073709551615", true, Some(json!(u64::MAX))),
            ("en", "18,446,744,073,709,551,616", true, None),
            ("en", "2.5", false, Some(json!(2.5))),
            ("en", "1,234.5", false, Some(json!(1234.5))),
            ("en", ".5", false, Some(json!(0.5))),
            ("en", "1.5", true, None),
            ("en", "1.0", true, None),
            ("en", "1,50", true, None),
            ("en", "1,2345", true, None),
            ("en", "12,345,67", true, None),
            ("en", ",500", true, None),
            ("en", "1,,500", true, None),
            ("en", "1.234,5", false, None),
            ("en", "1.2.3", false, None),
            ("en", "2.", false, None),
            ("en", "", true, None),
            ("en", "-", true, None),
            ("en", "abc", false, None),
            ("en", "inf", false, None),
            ("en", "1e5", false, None),
            ("de", "1500", true, Some(json!(1500))),
            ("de", "1.500", true, Some(json!(1500))),
            ("de", "1.234.567", true, Some(json!(1_234_567))),
            ("de", "1 500", true, Some(json!(1500))),
            ("de", "2,5", false, Some(json!(2.5))),
            ("de", "1.234,5", false, Some(json!(1234.5))),
            ("de", "-0,25", false, Some(json!(-0.25))),
            ("de", "1.50", true, None),
            ("de", "1.5", false, None),
            ("de", "2,5", true, None),
            ("de", "1,500", true, None),
            ("de", "1,234.5", false, None),
            ("de", "1.500,", false, None),
            ("fr", "1500", true, Some(json!(1500))),
            ("fr", "1\u{00A0}500", true, Some(json!(1500))),
            ("fr", "1\u{202F}234\u{202F}567", true, Some(json!(1_234_567))),
            ("fr", "1 500", true, Some(json!(1500))),
            ("fr", "2,5", false, Some(json!(2.5))),
            ("fr", "1\u{00A0}234,5", false, Some(json!(1234.5))),
            ("fr", "1\u{00A0}50", true, None),
            ("fr", "1.500", true, None),
            ("fr", "1.5", false, None),
            ("fr", "2,5", true, None),
            ("ja", "1,500", true, Some(json!(1500))),
            ("ja", "2.5", false, Some(json!(2.5))),
            ("ja", "2,5", false, None),
            ("ja", "1.500", true, None),
        ];
        for (lang, text, integer, want) in cases {
            assert_eq!(&parse_in(text, *integer, loc(lang)), want, "{lang}: {text:?} (whole: {integer})");
        }
    }

    #[test]
    fn quick_maths_is_read_in_the_language_style() {
        // (language, typed, current value, whole-number field, range, result)
        let r = |a: i64, b: i64| Some((json!(a), json!(b)));
        let cases: &[(&str, &str, Value, bool, Option<(Value, Value)>, Option<Value>)] = &[
            ("en", "+500", json!(1000), true, None, Some(json!(1500))),
            ("en", "+1,500", json!(10), true, None, Some(json!(1510))),
            ("en", "+ 1,500", json!(10), true, None, Some(json!(1510))),
            ("en", "*2", json!(21), true, None, Some(json!(42))),
            ("en", "x3", json!(5), true, None, Some(json!(15))),
            ("en", "\u{00D7}3", json!(5), true, None, Some(json!(15))),
            ("en", "/4", json!(10), true, None, Some(json!(3))),
            ("en", "/4", json!(-10), true, None, Some(json!(-3))),
            ("en", "/3", json!(10), true, None, Some(json!(3))),
            ("en", "/0", json!(10), true, None, None),
            ("en", "/0.0", json!(10), true, None, None),
            ("en", "/4", json!(10.0), false, None, Some(json!(2.5))),
            ("en", "+0.5", json!(1.25), false, None, Some(json!(1.75))),
            ("en", "+0.5", json!(1), true, None, None),
            ("en", "*1.5", json!(10), true, None, Some(json!(15))),
            ("en", "*0.5", json!(5), true, None, Some(json!(3))),
            ("en", "250k", json!(1), true, None, Some(json!(250_000))),
            ("en", "1.5m", json!(1), true, None, Some(json!(1_500_000))),
            ("en", "-2k", json!(1), true, None, Some(json!(-2000))),
            ("en", "+1.5K", json!(500), true, None, Some(json!(2000))),
            ("en", "1,000k", json!(1), true, None, Some(json!(1_000_000))),
            ("en", "1.2345k", json!(1), true, None, None),
            ("en", "1.2345k", json!(1.0), false, None, Some(json!(1234.5))),
            ("en", "max", json!(5), true, r(0, 99), Some(json!(99))),
            ("en", "MIN", json!(5), true, r(1, 99), Some(json!(1))),
            ("en", "max", json!(5), true, None, None),
            ("en", "+500", json!(90), true, r(0, 99), Some(json!(99))),
            ("en", "*1000", json!(-5), true, r(-100, 100), Some(json!(-100))),
            ("en", "+1.5k", json!(1), true, None, Some(json!(1501))),
            ("en", "+1,5k", json!(1), true, None, None),
            ("en", "-10", json!(50), true, None, None),
            ("en", "+abc", json!(1), true, None, None),
            ("en", "++5", json!(1), true, None, None),
            ("en", "*2", json!("text"), true, None, None),
            ("en", "km", json!(1), true, None, None),
            ("de", "+1.500", json!(10), true, None, Some(json!(1510))),
            ("de", "+1,5k", json!(0), true, None, Some(json!(1500))),
            ("de", "1,5m", json!(0), true, None, Some(json!(1_500_000))),
            ("de", "*2,5", json!(2.0), false, None, Some(json!(5.0))),
            ("de", "+0,5", json!(1.25), false, None, Some(json!(1.75))),
            ("de", "/2,5", json!(10.0), false, None, Some(json!(4.0))),
            ("de", "+1.5", json!(10), true, None, None),
            ("de", "+1.5k", json!(10), true, None, None),
            ("de", "+1,5", json!(10), true, None, None),
            ("fr", "+1\u{00A0}500", json!(0), true, None, Some(json!(1500))),
            ("fr", "+1 500", json!(0), true, None, Some(json!(1500))),
            ("fr", "2,5k", json!(0), true, None, Some(json!(2500))),
            ("fr", "*1,5", json!(10), true, None, Some(json!(15))),
            ("fr", "+1.5k", json!(0), true, None, None),
            ("ja", "+1,500", json!(0), true, None, Some(json!(1500))),
            ("ja", "1.5m", json!(0), true, None, Some(json!(1_500_000))),
            ("ja", "2,5k", json!(0), true, None, None),
            ("ja", "/2", json!(5), true, None, Some(json!(3))),
        ];
        for (lang, text, cur, integer, range, want) in cases {
            assert_eq!(&quick_maths_in(text, cur, *integer, range.clone(), loc(lang)), want, "{lang}: {text:?} on {cur}");
        }
    }

    /// The languages added later read numbers like the ones whose style they share, and
    /// what would be read as another number in another style is refused, never guessed.
    #[test]
    fn every_language_reads_its_own_style_and_refuses_the_others() {
        // (languages, typed, whole-number field, read as)
        let cases: &[(&[&str], &str, bool, Option<Value>)] = &[
            (&["it", "tr"], "1.500", true, Some(json!(1500))),
            (&["it", "tr"], "2,5", false, Some(json!(2.5))),
            (&["it", "tr"], "1.234.567,25", false, Some(json!(1_234_567.25))),
            (&["it", "tr"], "1.5", false, None),
            (&["it", "tr"], "1,500", true, None),
            (&["pl", "ru"], "1\u{00A0}500", true, Some(json!(1500))),
            (&["pl", "ru"], "1 234 567", true, Some(json!(1_234_567))),
            (&["pl", "ru"], "2,5", false, Some(json!(2.5))),
            (&["pl", "ru"], "1.500", true, None),
            (&["pl", "ru"], "2.5", false, None),
            (&["pl", "ru"], "1 50", true, None),
            (&["ko", "zh-TW", "zh-CN", "ja"], "1,500", true, Some(json!(1500))),
            (&["ko", "zh-TW", "zh-CN", "ja"], "2.5", false, Some(json!(2.5))),
            (&["ko", "zh-TW", "zh-CN", "ja"], "1.500", true, None),
            (&["ko", "zh-TW", "zh-CN", "ja"], "2,5", false, None),
            // Full-width digits and commas are refused rather than read as something else.
            (&["ja", "zh-CN", "zh-TW", "ko"], "\u{FF11}\u{FF15}\u{FF10}\u{FF10}", true, None),
            (&["zh-CN", "zh-TW"], "1\u{FF0C}500", true, None),
        ];
        for (langs, text, integer, want) in cases {
            for lang in *langs {
                assert_eq!(&parse_in(text, *integer, loc(lang)), want, "{lang}: {text:?} (whole: {integer})");
            }
        }
        // Quick maths on a whole-number field holding 10.
        for (lang, text, want) in [("tr", "+1.500", 1510), ("it", "2,5k", 2500), ("ru", "+1 500", 1510), ("pl", "*1,5", 15), ("ko", "+1,500", 1510), ("zh-TW", "1.5k", 1500)] {
            assert_eq!(quick_maths_in(text, &json!(10), true, None, loc(lang)), Some(json!(want)), "{lang}: {text:?}");
        }
    }

    #[test]
    fn whole_numbers_are_worked_out_exactly() {
        let q = |t: &str, cur: Value| quick_maths_in(t, &cur, true, None, POINT);
        // Above 2^53 a float can't hold every whole number: these must not go through one.
        let big = 9_007_199_254_740_993u64;
        assert_eq!(q("*1", json!(big)), Some(json!(big)));
        assert_eq!(q("x1", json!(big)), Some(json!(big)));
        assert_eq!(q("/1", json!(big)), Some(json!(big)));
        assert_eq!(q("+1", json!(big)), Some(json!(big + 1)));
        assert_eq!(q("*3", json!(3_000_000_000_000_000_001i64)), Some(json!(9_000_000_000_000_000_003u64)));
        assert_eq!(q("+1", json!(u64::MAX)), Some(json!(u64::MAX)), "pulled into what the type holds");
        assert_eq!(q("+1", json!(i64::MAX)), Some(json!(i64::MAX as u64 + 1)), "exact past i64");
        assert_eq!(q("*-1", json!(5)), Some(json!(-5)));
        assert_eq!(q("*-2", json!(i64::MIN)), Some(json!(u64::MAX)), "pulled into what the type holds");
        // Dividing by the most negative number that can be typed (no positive counterpart).
        assert_eq!(q("/-170141183460469231731687303715884105728", json!(5)), Some(json!(0)));
        assert_eq!(q("/-170141183460469231731687303715884105728", json!(i64::MIN)), Some(json!(0)));
        assert_eq!(q("/170141183460469231731687303715884105727", json!(u64::MAX)), Some(json!(0)));
        assert_eq!(q("/-170141183460469231731687303715884105729", json!(5)), None, "past what can be worked out");
    }

    #[test]
    fn results_keep_the_kind_of_number_the_field_holds() {
        let ints = Some((json!(0), json!(100)));
        let max = quick_maths_in("max", &json!(1.5), false, ints.clone(), POINT);
        assert_eq!(max, Some(json!(100.0)));
        assert!(max.as_ref().is_some_and(Value::is_f64), "a float field stays a float: {max:?}");
        let over = quick_maths_in("+500", &json!(1.5), false, ints.clone(), POINT);
        assert!(over.as_ref().is_some_and(Value::is_f64), "{over:?}");
        let min = quick_maths_in("min", &json!(7), true, ints, POINT);
        assert!(min.as_ref().is_some_and(|v| v.is_u64() || v.is_i64()), "{min:?}");
    }

    #[test]
    fn plain_numbers_and_negatives_are_not_maths() {
        // A plain negative number is typed and taken as it is, never "subtract".
        assert!(!is_quick_maths("-10"));
        assert!(!is_quick_maths("1500"));
        assert!(!is_quick_maths("2,500"));
        assert!(!is_quick_maths("1.234,5"));
        assert!(!is_quick_maths(""));
        assert!(!is_quick_maths("abc"));
        assert!(is_quick_maths("+10"));
        assert!(is_quick_maths("10k"));
        assert!(is_quick_maths(" max "));
    }

    #[test]
    fn edit_text_reads_back_as_the_same_number() {
        let half = Number::from_f64(2.5).expect("finite");
        assert_eq!(edit_text(&half, COMMA), "2,5");
        assert_eq!(edit_text(&half, SPACE), "2,5");
        assert_eq!(edit_text(&half, POINT), "2.5");
        assert_eq!(edit_text(&Number::from(1_234_567), COMMA), "1234567");
        for l in [POINT, COMMA, SPACE] {
            assert_eq!(parse_in(&edit_text(&half, l), false, l), Some(json!(2.5)));
            assert_eq!(parse_in(&edit_text(&Number::from(-1_234_567), l), true, l), Some(json!(-1_234_567)));
        }
    }

    #[test]
    fn digits_group_by_language_separator() {
        assert_eq!(group_digits("3520485254", ','), "3,520,485,254");
        assert_eq!(group_digits("-1234567", '.'), "-1.234.567");
        assert_eq!(group_digits("100000", '\u{00A0}'), "100\u{00A0}000");
        assert_eq!(group_digits("2024", ','), "2024", "small numbers stay plain");
        assert_eq!(group_digits("-9999", ','), "-9999");
        assert_eq!(group_digits("10000", ','), "10,000");
        assert_eq!(group_digits("1.5", ','), "1.5");
        assert_eq!(group_digits("18446744073709551615", ','), "18,446,744,073,709,551,615");
        // Grouped text reads back as the same number in its own language.
        for l in [POINT, COMMA, SPACE] {
            assert_eq!(parse_in(&group_digits("3520485254", l.group), true, l), Some(json!(3_520_485_254u64)));
        }
    }

    #[test]
    fn type_ranges_are_told_from_real_caps() {
        let r = |a: Value, b: Value| is_type_range(&a, &b);
        assert!(r(json!(i32::MIN), json!(i32::MAX)));
        assert!(r(json!(0), json!(u32::MAX)));
        assert!(r(json!(0), json!(255)));
        assert!(r(json!(i64::MIN), json!(i64::MAX)));
        assert!(r(json!(0), json!(u64::MAX)));
        assert!(r(json!(0), json!(i32::MAX)));
        assert!(r(json!(-(f32::MAX as f64)), json!(f32::MAX as f64)));
        assert!(!r(json!(1), json!(99)));
        assert!(!r(json!(0), json!(999)));
        assert!(!r(json!(0), json!(9_999_999)));
        assert!(!r(json!(0.0), json!(100.0)));
    }

    #[test]
    fn bit_widths_are_told_from_game_caps() {
        let b = |a: i64, c: i64| is_bit_width(&json!(a), &json!(c));
        assert!(b(0, 1023));
        assert!(b(0, 33_554_431));
        assert!(b(-128, 127));
        assert!(!b(0, 99));
        assert!(!b(1, 99));
        assert!(!b(0, 999_999));
        assert!(!b(0, 1), "an on/off field");
        assert!(!b(0, 15), "too small to be anything but a real limit");
    }
}
