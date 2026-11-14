//! JSON with bare values inside objects (Inscryption writes a Vector2 as `{ "$type": "...", 0.5, 0.42 }`).
//! Each bare value is given a made-up key `"$bare0"`, `"$bare1"`… while editing and the keys are dropped again on saving.

const KEY: &str = "\"$bare";

pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let t = std::str::from_utf8(data).ok()?;
    if !t.trim_start().starts_with('{') || serde_json::from_str::<serde_json::Value>(t).is_ok() {
        return None;
    }
    let b = t.as_bytes();
    let mut out = String::with_capacity(t.len() + 1024);
    let mut stack: Vec<u8> = Vec::new();
    let mut want_key = false; // inside an object, a key (or `}`) is due next
    let mut n = 0usize;
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                want_key = false;
            }
            b'{' => {
                stack.push(b'{');
                want_key = true;
            }
            b'[' => {
                stack.push(b'[');
                want_key = false;
            }
            b'}' | b']' => {
                stack.pop();
                want_key = false;
            }
            b',' => want_key = stack.last() == Some(&b'{'),
            b':' => want_key = false,
            c if c.is_ascii_whitespace() => {}
            _ => {
                if c == b'$' && !want_key {
                    // a bare reference such as `$iref:184`: shown as a string
                    let mut j = i;
                    while j < b.len() && !matches!(b[j], b',' | b'}' | b']' | b'\r' | b'\n' | b' ') {
                        j += 1;
                    }
                    out.push_str(&t[last..i]);
                    out.push('"');
                    out.push_str(&t[i..j]);
                    out.push('"');
                    n += 1;
                    last = j;
                    i = j;
                    continue;
                }
                if want_key {
                    // a bare value where a key should be
                    let mut j = i;
                    while j < b.len() && !matches!(b[j], b',' | b'}' | b'\r' | b'\n') {
                        j += 1;
                    }
                    out.push_str(&t[last..i]);
                    out.push_str(&format!("{KEY}{n}\": "));
                    n += 1;
                    last = i;
                    i = j;
                    want_key = false;
                    continue;
                }
            }
        }
        i += 1;
    }
    if n == 0 {
        return None;
    }
    out.push_str(&t[last..]);
    serde_json::from_str::<serde_json::Value>(&out).ok()?;
    Some(out.into_bytes())
}

fn unquote_refs(t: &str) -> String {
    const R: &str = "\"$iref:";
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(p) = rest.find(R) {
        out.push_str(&rest[..p]);
        let after = &rest[p + 1..];
        let end = after.find('"').unwrap_or(after.len());
        let body = &after[..end];
        if body.len() > 6 && body[6..].bytes().all(|c| c.is_ascii_digit()) && end < after.len() {
            out.push_str(body);
            rest = &after[end + 1..];
        } else {
            out.push('"');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

pub fn pack(plain: &[u8]) -> Vec<u8> {
    let t = String::from_utf8_lossy(plain);
    let mut out = String::with_capacity(t.len());
    let mut rest: &str = &t;
    while let Some(p) = rest.find(KEY) {
        out.push_str(&rest[..p]);
        let after = &rest[p + KEY.len()..];
        // `"$bare<digits>": `
        let digits = after.bytes().take_while(|c| c.is_ascii_digit()).count();
        let tail = &after[digits..];
        if tail.starts_with("\": ") {
            rest = &tail[3..];
        } else if tail.starts_with("\":") {
            rest = &tail[2..];
        } else {
            out.push_str(KEY);
            rest = after;
        }
    }
    out.push_str(rest);
    unquote_refs(&out).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_values_round_trip() {
        let src = b"{\n  \"a\": {\n    \"$type\": \"V\",\n    0.5,\n    0.42\n  },\n  \"b\": [\n    $iref:3,\n    $iref:4\n  ]\n}";
        let plain = open(src).unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&plain).is_ok());
        assert_eq!(pack(&plain), src);
    }

    #[test]
    fn plain_json_is_left_alone() {
        assert!(open(b"{\"a\": 1}").is_none());
    }
}

