use core::str;
use std::{char, fmt::Display, string::FromUtf16Error};

use crate::{error::Error, parser::Reader};

/// A JSON string is just a list of 16-bit values.
///
/// They are often valid UTF-16 strings, however they can contain lonely
/// [surrogate code points](https://www.unicode.org/glossary/#surrogate_code_point).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonString {
    inner: Vec<u16>,
}

pub struct InvalidUnicodeString(pub JsonString);

fn parse_hex_byte(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::InvalidHexChar(byte)),
    }
}

fn parse_hex_escape(bytes: &[u8; 4]) -> Result<u16, Error> {
    let mut r = 0;
    for &b in bytes {
        r *= 16;
        r += u16::from(parse_hex_byte(b)?);
    }
    Ok(r)
}

pub(crate) fn read_string(reader: &mut Reader) -> Result<JsonString, Error> {
    let mut inner = Vec::new();
    match reader.read_byte()? {
        b'"' => {}
        b => {
            return Err(Error::ExpectedDoubleQuote(b));
        }
    }

    loop {
        match reader.peek_byte().ok_or(Error::UnexpectedEof)? {
            b'\\' => {
                reader.read_byte().unwrap();
                let v = match reader.read_byte()? {
                    b'"' => b'"',
                    b'\\' => b'\\',
                    b'/' => b'/',
                    b'b' => 0x08,
                    b'f' => 0x0c,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'u' => {
                        let hex = reader.read_bytes::<4>()?;
                        let v = parse_hex_escape(hex)?;
                        inner.push(v);
                        continue;
                    }
                    b => return Err(Error::UnexpectedEscape(b)),
                };
                inner.push(v.into());
            }
            b'"' => {
                reader.read_byte().unwrap();
                break;
            }
            b => {
                if b < 0x20 {
                    return Err(Error::InvalidControlCharacter(b));
                }
                let char = reader.read_char()?;
                let mut buf = [0; 2];
                inner.extend_from_slice(char.encode_utf16(&mut buf));
            }
        }
    }

    Ok(JsonString { inner })
}

impl Display for JsonString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;

        for c in char::decode_utf16(self.inner.iter().copied()) {
            match c {
                Ok(c) => {
                    let escape_char = match c {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        '\x08' => 'b',
                        '\x0c' => 'f',
                        '\n' => 'n',
                        '\r' => 'r',
                        '\t' => 't',
                        '\x00'..'\x20' => {
                            write!(f, "\\u{:04x}", u32::from(c))?;
                            continue;
                        }
                        _ => {
                            write!(f, "{c}")?;
                            continue;
                        }
                    };
                    write!(f, "\\{escape_char}")?;
                }
                Err(e) => write!(f, "\\u{:04x}", e.unpaired_surrogate())?,
            }
        }

        write!(f, "\"")
    }
}

impl JsonString {
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Reader::read_all(bytes, read_string)
    }

    pub fn as_string(&self) -> Option<String> {
        String::from_utf16(&self.inner).ok()
    }

    pub fn as_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.inner)
    }
}

impl From<&str> for JsonString {
    fn from(value: &str) -> Self {
        Self {
            inner: value.encode_utf16().collect(),
        }
    }
}

impl From<String> for JsonString {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl TryFrom<&JsonString> for String {
    type Error = FromUtf16Error;

    fn try_from(value: &JsonString) -> Result<Self, Self::Error> {
        String::from_utf16(&value.inner)
    }
}

impl TryFrom<JsonString> for String {
    type Error = InvalidUnicodeString;

    fn try_from(value: JsonString) -> Result<Self, Self::Error> {
        (&value).try_into().map_err(|_| InvalidUnicodeString(value))
    }
}

impl From<JsonString> for Vec<u16> {
    fn from(value: JsonString) -> Self {
        value.inner
    }
}

impl From<Vec<u16>> for JsonString {
    fn from(value: Vec<u16>) -> Self {
        Self { inner: value }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unicode() {
        assert_eq!(
            JsonString::from_json(r#""鑅""#.as_bytes())
                .unwrap()
                .as_string()
                .unwrap(),
            "鑅"
        );
    }

    #[test]
    fn test_unicode_2() {
        assert_eq!(
            JsonString::from_json(r#""鑅""#.as_bytes())
                .unwrap()
                .to_string(),
            r#""鑅""#
        );
    }

    #[test]
    fn test_lone_surrogate() {
        assert_eq!(
            JsonString::from_json(br#""\ud800""#),
            Ok(vec![0xd800].into())
        );
        assert_eq!(
            JsonString::from_json(br#""\ud800""#).unwrap().to_string(),
            r#""\ud800""#
        );
    }
}
