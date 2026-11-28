//! What a number box shows and accepts beyond plain digits: digit grouping while the box is not
//! being typed in, and quick maths (`+500`, `*2`, `max`, `250k`) worked out when the typing is
//! done. Neither changes how a plain number is read or stored.

use serde_json::{Number, Value};

/// The digit-group separator of the interface language: "1,234,567" in English, Japanese and
/// Chinese, "1.234.567" in German, Spanish and Portuguese, "1 234 567" in French (a no-break
/// space, so the number never splits over two lines).
pub fn group_separator() -> char {
    match crate::i18n::current().code {
        "de" | "es" | "pt-BR" => '.',
        "fr" => '\u{00A0}',
        _ => ',',
    }
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

/// Whether typed text is quick maths (worked out when the box is left or Enter is pressed)
/// rather than a plain number (taken while typing). A leading "-" is always a plain negative
/// number, never "subtract": typing -5 into a signed field must give -5.
pub fn is_quick_maths(text: &str) -> bool {
    let t = squeeze(text);
    if t.is_empty() {
        return false;
    }
    if matches!(t.as_str(), "max" | "min") {
        return true;
    }
    t.starts_with(['+', '*', 'x', '\u{00D7}', '/']) || t.ends_with(['k', 'm'])
}

/// The text without spaces, group commas and underscores, in lower case.
fn squeeze(text: &str) -> String {
    text.chars().filter(|c| !matches!(c, ' ' | ',' | '_' | '\u{00A0}' | '\u{202F}')).flat_map(char::to_lowercase).collect()
}

/// "250k", "1.5m", "12": the amount, with k meaning thousands and m millions.
fn amount(t: &str) -> Option<f64> {
    let (num, mul) = if let Some(n) = t.strip_suffix('k') {
        (n, 1e3)
    } else if let Some(n) = t.strip_suffix('m') {
        (n, 1e6)
    } else {
        (t, 1.0)
    };
    if num.is_empty() || num.starts_with('+') {
        return None;
    }
    let x: f64 = num.parse().ok()?;
    let x = x * mul;
    x.is_finite().then_some(x)
}

/// Works out quick maths typed into a number box holding `current`:
/// `+N` adds, `*N` (or `xN`) multiplies, `/N` divides, `max` and `min` take the ends of
/// `range` (the values the field accepts), and a `k` or `m` after a number means thousands
/// or millions (`250k`, `-2k`, `+1.5m`). The result is pulled into `range`; a whole-number
/// field gets the nearest whole number. `None`: not quick maths, or nothing sensible came
/// out (dividing by zero, `max` without a known range, a field that isn't a number).
pub fn quick_maths(text: &str, current: &Value, integer: bool, range: Option<(Value, Value)>) -> Option<Value> {
    let t = squeeze(text);
    if !is_quick_maths(&t) {
        return None;
    }
    match t.as_str() {
        "max" => return range.map(|r| r.1),
        "min" => return range.map(|r| r.0),
        _ => {}
    }
    let cur = || current.as_f64();
    let result = if let Some(rest) = t.strip_prefix('+') {
        let n = amount(rest)?;
        // Whole numbers are added exactly (a 64-bit value would lose its last digits as a float).
        if integer && n.fract() == 0.0 {
            return whole_sum(current, n, range);
        }
        cur()? + n
    } else if let Some(rest) = t.strip_prefix(['*', 'x', '\u{00D7}']) {
        cur()? * amount(rest)?
    } else if let Some(rest) = t.strip_prefix('/') {
        let n = amount(rest)?;
        if n == 0.0 {
            return None;
        }
        cur()? / n
    } else {
        amount(&t)?
    };
    to_value(result, integer, range)
}

/// `current + n` for a whole-number field, without going through a float.
fn whole_sum(current: &Value, n: f64, range: Option<(Value, Value)>) -> Option<Value> {
    let base = current.as_i64().map(i128::from).or_else(|| current.as_u64().map(i128::from))?;
    if n.abs() >= 1e30 {
        return None;
    }
    let sum = base.saturating_add(n as i128);
    let v = if let Ok(i) = i64::try_from(sum) {
        Value::from(i)
    } else if let Ok(u) = u64::try_from(sum) {
        Value::from(u)
    } else if sum < 0 {
        Value::from(i64::MIN)
    } else {
        Value::from(u64::MAX)
    };
    Some(crate::widgets::clamp_to(range, v))
}

fn to_value(x: f64, integer: bool, range: Option<(Value, Value)>) -> Option<Value> {
    if !x.is_finite() {
        return None;
    }
    let v = if integer {
        let r = x.round();
        if r >= 0.0 && r > i64::MAX as f64 {
            Value::from(if r >= u64::MAX as f64 { u64::MAX } else { r as u64 })
        } else {
            Value::from(r.max(i64::MIN as f64) as i64)
        }
    } else {
        Value::Number(Number::from_f64(x)?)
    };
    Some(crate::widgets::clamp_to(range, v))
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

    fn q(text: &str, cur: Value, range: Option<(i64, i64)>) -> Option<Value> {
        quick_maths(text, &cur, true, range.map(|(a, b)| (Value::from(a), Value::from(b))))
    }

    #[test]
    fn adds_multiplies_and_divides() {
        assert_eq!(q("+500", json!(1000), None), Some(json!(1500)));
        assert_eq!(q("+ 1,500", json!(10), None), Some(json!(1510)));
        assert_eq!(q("*2", json!(21), None), Some(json!(42)));
        assert_eq!(q("x3", json!(5), None), Some(json!(15)));
        assert_eq!(q("/4", json!(10), None), Some(json!(3)), "whole-number fields round to the nearest");
        assert_eq!(q("/0", json!(10), None), None);
        assert_eq!(quick_maths("/4", &json!(10.0), false, None), Some(json!(2.5)));
        assert_eq!(quick_maths("+0.5", &json!(1.25), false, None), Some(json!(1.75)));
    }

    #[test]
    fn suffixes_and_ends_of_the_range() {
        assert_eq!(q("250k", json!(1), None), Some(json!(250_000)));
        assert_eq!(q("1.5m", json!(1), None), Some(json!(1_500_000)));
        assert_eq!(q("-2k", json!(1), None), Some(json!(-2000)));
        assert_eq!(q("+1.5K", json!(500), None), Some(json!(2000)));
        assert_eq!(q("max", json!(5), Some((0, 99))), Some(json!(99)));
        assert_eq!(q("MIN", json!(5), Some((1, 99))), Some(json!(1)));
        assert_eq!(q("max", json!(5), None), None, "no known top");
    }

    #[test]
    fn results_stay_in_range() {
        assert_eq!(q("+500", json!(90), Some((0, 99))), Some(json!(99)));
        assert_eq!(q("*1000", json!(-5), Some((-100, 100))), Some(json!(-100)));
        assert_eq!(q("+1", json!(u64::MAX), None), Some(json!(u64::MAX)));
        assert_eq!(q("+1", json!(i64::MAX), None), Some(json!(i64::MAX as u64 + 1)), "exact past i64");
        assert_eq!(q("+1", json!(9_007_199_254_740_993u64), None), Some(json!(9_007_199_254_740_994u64)), "no float rounding");
    }

    #[test]
    fn plain_numbers_and_negatives_are_not_maths() {
        // A plain negative number is typed and taken as it is, never "subtract".
        assert!(!is_quick_maths("-10"));
        assert!(!is_quick_maths("1500"));
        assert!(!is_quick_maths("2,500"));
        assert!(!is_quick_maths(""));
        assert!(!is_quick_maths("abc"));
        assert_eq!(q("-10", json!(50), None), None);
        assert!(is_quick_maths("+10"));
        assert!(is_quick_maths("10k"));
        assert!(is_quick_maths(" max "));
        // Not a number at all.
        assert_eq!(q("+abc", json!(1), None), None);
        assert_eq!(q("++5", json!(1), None), None);
        assert_eq!(q("*2", json!("text"), None), None);
        assert_eq!(q("km", json!(1), None), None);
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
        // Grouped text with commas still reads back as the same number.
        assert_eq!(crate::widgets::parse_number(&group_digits("3520485254", ','), true), Some(json!(3_520_485_254u64)));
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
