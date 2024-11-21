use std::{fmt::Display, hash::Hash, str};

use crate::{error::Error, parser::Reader};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Number {
    inner: f64,
}

impl Eq for Number {}

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Number {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Hash for Number {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.to_bits().hash(state);
    }
}

impl Number {
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Reader::read_all(bytes, read_number)
    }
}

fn skip_digits(reader: &mut Reader) -> Result<bool, Error> {
    let mut found_digit = false;
    while let Some(b'0'..=b'9') = reader.peek_byte() {
        reader.read_byte()?;
        found_digit = true;
    }
    Ok(found_digit)
}

fn skip_number(reader: &mut Reader) -> Result<(), Error> {
    match reader.peek_byte() {
        None => return Err(Error::UnexpectedEof),
        Some(b'-') => {
            reader.read_byte()?;
        }
        _ => {}
    }

    let b = reader.read_byte()?;
    match b {
        b'0' => {}
        b'1'..=b'9' => {
            skip_digits(reader)?;
        }
        _ => return Err(Error::InvalidDigit(b)),
    }

    if reader.peek_byte() == Some(b'.') {
        reader.read_byte()?;

        if !skip_digits(reader)? {
            return Err(Error::InvalidDigit(reader.read_byte()?));
        }
    }

    if let Some(b'e' | b'E') = reader.peek_byte() {
        reader.read_byte()?;

        if let Some(b'+' | b'-') = reader.peek_byte() {
            reader.read_byte()?;
        }

        if !skip_digits(reader)? {
            return Err(Error::InvalidDigit(reader.read_byte()?));
        }
    }

    Ok(())
}

// TODO: Add support for integers
// Hard cases:
// `0.123e3` -> `123u64`
// `1000000000000000000000000000e-10` -> `100000000000000000u64`
pub(crate) fn read_number(reader: &mut Reader) -> Result<Number, Error> {
    let (slice, _) = reader.parse_slice(skip_number)?;
    let s = str::from_utf8(slice).unwrap();
    let v: f64 = s.parse().unwrap();

    if !v.is_finite() {
        return Err(Error::InfiniteFloat);
    }

    Ok(Number { inner: v })
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl TryFrom<f64> for Number {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Self { inner: value })
        } else {
            Err(())
        }
    }
}

impl TryFrom<f32> for Number {
    type Error = ();

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        f64::from(value).try_into()
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<serde_json::Number> for Number {
    type Error = crate::InvalidSerdeJsonNumber;

    fn try_from(value: serde_json::Number) -> Result<Self, Self::Error> {
        let Some(value) = value.as_f64() else {
            return Err(crate::InvalidSerdeJsonNumber(value));
        };
        Ok(value.try_into().unwrap())
    }
}

#[cfg(feature = "serde_json")]
impl From<Number> for serde_json::Number {
    fn from(value: Number) -> Self {
        serde_json::Number::from_f64(value.inner).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_int() {
        assert_eq!(Number::from_json(b"123"), Ok(Number { inner: 123.0 }));
    }

    #[test]
    fn test_parse_fractional() {
        assert_eq!(Number::from_json(b"1.23"), Ok(Number { inner: 1.23 }));
    }

    #[test]
    fn test_parse_full() {
        assert_eq!(Number::from_json(b"0.12e50"), Ok(Number { inner: 0.12e50 }));
    }

    #[test]
    fn test_parse_inf() {
        assert_eq!(Number::from_json(b"1e400"), Err(Error::InfiniteFloat));
    }
}
