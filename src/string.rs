use core::str;
use std::fmt::Display;

use wtf8::{CodePoint, Wtf8Buf};

use crate::{error::Error, parser::Reader};

/// A JSON string is just a list of 16-bit values.
///
/// They are often valid UTF-16 strings, however they can contain lonely
/// [surrogate code points](https://www.unicode.org/glossary/#surrogate_code_point).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonString {
    inner: Wtf8Buf,
}

impl Default for JsonString {
    fn default() -> Self {
        Self {
            inner: Wtf8Buf::new(),
        }
    }
}

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

fn u16_to_code_point(v: u16) -> CodePoint {
    CodePoint::from_u32(v.into()).unwrap()
}

fn u8_to_code_point(v: u8) -> CodePoint {
    CodePoint::from_u32(v.into()).unwrap()
}

pub(crate) fn read_string(reader: &mut Reader) -> Result<JsonString, Error> {
    let mut inner = Wtf8Buf::new();
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
                        inner.push(u16_to_code_point(v));
                        continue;
                    }
                    b => return Err(Error::UnexpectedEscape(b)),
                };
                inner.push(u8_to_code_point(v));
            }
            b'"' => {
                reader.read_byte().unwrap();
                break;
            }
            b => {
                if b < 0x20 {
                    return Err(Error::InvalidControlCharacter(b));
                }
                inner.push_char(reader.read_char()?);
            }
        }
    }

    Ok(JsonString { inner })
}

impl Display for JsonString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;

        for c in self.inner.code_points() {
            match c.to_char() {
                Some(c) => {
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
                None => write!(f, "\\u{:04x}", c.to_u32())?,
            }
        }

        write!(f, "\"")
    }
}

impl JsonString {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Reader::read_all(bytes, read_string)
    }

    pub fn from_ill_formed_utf16(v: &[u16]) -> Self {
        Self {
            inner: Wtf8Buf::from_ill_formed_utf16(v),
        }
    }

    pub fn into_string(self) -> Result<String, Self> {
        self.inner
            .into_string()
            .map_err(|inner| JsonString { inner })
    }

    pub fn into_string_lossy(self) -> String {
        self.inner.into_string_lossy()
    }

    pub fn as_str(&self) -> Option<&str> {
        self.inner.as_str()
    }

    pub fn to_ill_formed_utf16(&self) -> impl Iterator<Item = u16> + '_ {
        self.inner.to_ill_formed_utf16()
    }
}

impl From<&str> for JsonString {
    fn from(value: &str) -> Self {
        Self {
            inner: Wtf8Buf::from_str(value),
        }
    }
}

impl From<String> for JsonString {
    fn from(value: String) -> Self {
        Self {
            inner: Wtf8Buf::from_string(value),
        }
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
                .as_str()
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
            JsonString::from_json(br#""\ud800""#)
                .unwrap()
                .to_ill_formed_utf16()
                .collect::<Vec<_>>(),
            vec![0xd800]
        );
        assert_eq!(
            JsonString::from_json(br#""\ud800""#).unwrap().to_string(),
            r#""\ud800""#
        );
    }
}
