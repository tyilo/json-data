mod array;
mod error;
mod number;
mod object;
mod parser;
mod string;

use std::{collections::BTreeMap, fmt::Display};

use crate::{
    array::{read_array, Array},
    error::Error,
    number::{read_number, Number},
    object::{read_object, Object},
    parser::Reader,
    string::{read_string, JsonString},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(JsonString),
    Array(Array),
    Object(Object),
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl TryFrom<f64> for Value {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Ok(Self::Number(value.try_into()?))
    }
}

impl From<JsonString> for Value {
    fn from(value: JsonString) -> Self {
        Self::String(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<Array> for Value {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Array(value.into())
    }
}

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl From<BTreeMap<JsonString, Value>> for Value {
    fn from(value: BTreeMap<JsonString, Value>) -> Self {
        Self::Object(value.into())
    }
}

fn read_value(reader: &mut Reader) -> Result<Value, Error> {
    reader.skip_whitespace();

    let Some(b) = reader.peek_byte() else {
        return Err(Error::UnexpectedEof);
    };

    let v = match b {
        b'n' => {
            if reader.read_bytes::<4>()? != b"null" {
                return Err(Error::ExpectedNull);
            }
            Value::Null
        }
        b'f' => {
            if reader.read_bytes::<5>()? != b"false" {
                return Err(Error::ExpectedFalse);
            }
            Value::Bool(false)
        }
        b't' => {
            if reader.read_bytes::<4>()? != b"true" {
                return Err(Error::ExpectedTrue);
            }
            Value::Bool(true)
        }
        b'-' | b'0'..=b'9' => Value::Number(read_number(reader)?),
        b'"' => Value::String(read_string(reader)?),
        b'[' => Value::Array(read_array(reader)?),
        b'{' => Value::Object(read_object(reader)?),
        _ => return Err(Error::UnexpectedStartOfValue(b)),
    };

    reader.skip_whitespace();
    Ok(v)
}

impl Value {
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Reader::read_all(bytes, read_value)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Number(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v}"),
            Value::Array(v) => write!(f, "{v}"),
            Value::Object(v) => write!(f, "{v}"),
        }
    }
}

#[cfg(feature = "serde_json")]
pub use serde_json;

#[cfg(feature = "serde_json")]
pub struct InvalidSerdeJsonNumber(pub serde_json::Number);

#[cfg(feature = "serde_json")]
impl TryFrom<serde_json::Value> for Value {
    type Error = InvalidSerdeJsonNumber;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        Ok(match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(v) => Value::Number(v.try_into()?),
            serde_json::Value::String(s) => Value::String(JsonString::from(s)),
            serde_json::Value::Array(arr) => Value::Array(Array::try_from(arr)?),
            serde_json::Value::Object(map) => Value::Object(Object::try_from(map)?),
        })
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<Value> for serde_json::Value {
    type Error = crate::string::InvalidUnicodeString;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Number(v) => serde_json::Value::Number(v.into()),
            Value::String(s) => serde_json::Value::String(s.try_into()?),
            Value::Array(arr) => serde_json::Value::Array(arr.try_into()?),
            Value::Object(obj) => serde_json::Value::Object(obj.try_into()?),
        })
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use super::*;

    prop_compose! {
        fn interesting_u16()(i in 0..17usize) -> u16 {
            let arr: [u16; 17] = [
                // Backslash escaped:
                b'"' as u16,
                b'\\' as u16,
                b'/' as u16,
                0x08,
                0x0c,
                b'\n' as u16,
                b'\r' as u16,
                b'\t' as u16,
                // Hex escaped:
                0x00,
                0x19,
                // Below surrogates:
                0xD7FF,
                // Surrogates:
                0xD800,
                0xDBFF,
                0xDC00,
                0xDFFF,
                // Above surrogates:
                0xE000,
                // Max:
                u16::MAX,
            ];
            arr[i]
        }
    }

    fn interesting_arb_string() -> impl Strategy<Value = JsonString> {
        prop::collection::vec(interesting_u16(), 0..10).prop_map(JsonString::from)
    }

    fn arb_string() -> impl Strategy<Value = JsonString> {
        any::<Vec<u16>>().prop_map(JsonString::from)
    }

    fn arb_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<f64>().prop_filter_map("non-finite f64", |v| Value::try_from(v).ok()),
            arb_string().prop_map(Value::String),
        ];

        leaf.prop_recursive(8, 256, 10, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..10).prop_map(Value::from),
                prop::collection::btree_map(arb_string(), inner, 0..10).prop_map(Value::from),
            ]
        })
    }

    proptest! {
        #[test]
        fn test_value_to_string_and_back(value in arb_value()) {
            let s = value.to_string();
            let v2 = Value::from_json(s.as_bytes());
            assert_eq!(v2, Ok(value));
        }

        #[test]
        fn test_string_to_string_and_back(s in arb_string()) {
            let json_s = s.to_string();
            let s2 = JsonString::from_json(json_s.as_bytes());
            assert_eq!(s2, Ok(s));
        }

        #[test]
        fn test_interesting_string_to_string_and_back(s in interesting_arb_string()) {
            let json_s = s.to_string();
            let s2 = JsonString::from_json(json_s.as_bytes());
            assert_eq!(s2, Ok(s));
        }
    }
}
