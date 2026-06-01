//! Exact reimplementation of the original SQF array serializer (`src/Epochlib/SQF.cpp`).
//!
//! This is the single most critical module for backward compatibility.
//! Every string returned by the extension is built through this code and then
//! `call compile`'d or `parseSimpleArray`'d by the SQF hive wrappers.
//!
//! Contract (from original):
//! - `to_array()` produces `"[elem1,elem2,...]"` with **no spaces**.
//! - `push_str(s, 0)` → `"\"s\""` (double quotes, **no** inner escaping of " or \).
//! - `push_str(s, 1)` → `"'s'"` (single quotes) **plus** the special '→'' doubling
//!   performed in the three GET paths only.
//! - `push_number(n)` → bare decimal digits (no quotes).
//! - `push_array("[json...]")` → the string **verbatim** (used for Redis values that already look like arrays).
//! - NULL / missing → literal `nil` (unquoted).
//!
//! The only place ' doubling occurs is inside `get`/`get_ttl`/`get_range` when
//! they build the chunk for `push_str(..., 1)`.

// (no longer needed after simplification)

#[derive(Clone, Debug, PartialEq)]
pub enum SQFValue {
    Str(String, bool), // (content, single_quote_flag)
    Number(String),    // pre-formatted (for exact byte slices from strftime, etc.)
    #[allow(dead_code)]
    // Used in unit tests today. Also reserved for future use when we want to
    // pass through raw Redis array values or represent SQF `nil` from non-test paths.
    ArrayLiteral(String), // verbatim (e.g. a Redis value starting with '[')
    #[allow(dead_code)]
    // Used in unit tests today. Reserved for future paths that need to emit SQF `nil`.
    Nil,
}

#[derive(Default, Clone, Debug)]
pub struct SQF {
    elements: Vec<SQFValue>,
}

impl SQF {
    pub fn new() -> Self {
        Self::default()
    }

    /// push_str with flag (0 = double quotes, 1 = single quotes + special ' doubling in callers).
    /// The doubling of ' is **not** done here — it is done by the caller (get/get_ttl/get_range)
    /// exactly as the original C++ did, then the already-processed string is pushed with flag=1.
    pub fn push_str(&mut self, s: &str, flag: i32) {
        if flag == 1 {
            self.elements.push(SQFValue::Str(s.to_owned(), true));
        } else {
            self.elements.push(SQFValue::Str(s.to_owned(), false));
        }
    }

    pub fn push_str_double(&mut self, s: &str) {
        self.push_str(s, 0);
    }

    /// Bare number (or pre-formatted slice for getCurrentTime).
    /// Accepts anything Display-able or pre-formatted strings (matches original usage).
    pub fn push_number<T: std::fmt::Display>(&mut self, n: T) {
        self.elements.push(SQFValue::Number(n.to_string()));
    }

    /// Verbatim array element (used when Redis already returned something starting with '[').
    #[cfg(test)]
    pub fn push_array_literal(&mut self, s: &str) {
        self.elements.push(SQFValue::ArrayLiteral(s.to_owned()));
    }

    #[cfg(test)]
    pub fn push_nil(&mut self) {
        self.elements.push(SQFValue::Nil);
    }

    /// Exact reproduction of the original `toArray()` logic.
    pub fn to_array(&self) -> String {
        let mut out = String::with_capacity(64);
        out.push('[');

        for (i, v) in self.elements.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            match v {
                SQFValue::Str(s, single) => {
                    let q = if *single { '\'' } else { '"' };
                    // NOTE: original does **no** escaping of inner quotes or backslashes.
                    // Only the special ' doubling is performed by the three GET callers
                    // before calling push_str(..., 1).
                    out.push(q);
                    out.push_str(s);
                    out.push(q);
                }
                SQFValue::Number(n) => out.push_str(n),
                SQFValue::ArrayLiteral(a) => out.push_str(a),
                SQFValue::Nil => out.push_str("nil"),
            }
        }

        out.push(']');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let s = SQF::new();
        assert_eq!(s.to_array(), "[]");
    }

    #[test]
    fn basic_mixed() {
        let mut s = SQF::new();
        s.push_str_double("hello");
        s.push_number(42i64);
        s.push_str("world", 1); // single-quoted (caller already doubled ' if needed)
        s.push_nil();
        s.push_array_literal("[1,2,3]");
        assert_eq!(s.to_array(), r#"["hello",42,'world',nil,[1,2,3]]"#);
    }

    #[test]
    fn number_from_char_slice_like_original_getcurrenttime() {
        let mut s = SQF::new();
        s.push_number("2026"); // as the original does with strftime + assign(size)
        s.push_number("05");
        s.push_number("29");
        assert_eq!(s.to_array(), "[2026,05,29]");
    }

    #[test]
    fn single_quote_doubling_is_caller_responsibility() {
        // This matches the original: the GET paths build the escaped string themselves.
        let mut s = SQF::new();
        let mut escaped = String::new();
        for ch in "it's".chars() {
            if ch == '\'' {
                escaped.push('\'');
            }
            escaped.push(ch);
        }
        s.push_str(&escaped, 1);
        assert_eq!(s.to_array(), r#"['it''s']"#);
    }
}
