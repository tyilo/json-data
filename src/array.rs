use std::fmt::Display;

use crate::{error::Error, parser::Reader, read_value, Value};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Array {
    inner: Vec<Value>,
}

impl Array {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Reader::read_all(bytes, read_array)
    }
}

impl From<Vec<Value>> for Array {
    fn from(value: Vec<Value>) -> Self {
        Self { inner: value }
    }
}

pub(crate) fn read_array(reader: &mut Reader) -> Result<Array, Error> {
    match reader.read_byte()? {
        b'[' => {}
        b => {
            return Err(Error::ExpectedLeftBracket(b));
        }
    }

    reader.skip_whitespace();
    if reader.peek_byte() == Some(b']') {
        reader.read_byte()?;
        return Ok(Array::default());
    }

    let mut inner = Vec::new();
    loop {
        inner.push(read_value(reader)?);

        match reader.read_byte()? {
            b']' => break,
            b',' => {}
            b => return Err(Error::ExpectedCommaOrRightBracket(b)),
        }
    }

    Ok(Array { inner })
}

impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.inner.iter().enumerate() {
            if i != 0 {
                write!(f, ",")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "]")
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = <Vec<Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<Vec<serde_json::Value>> for Array {
    type Error = crate::InvalidSerdeJsonNumber;

    fn try_from(value: Vec<serde_json::Value>) -> Result<Self, Self::Error> {
        Ok(value
            .into_iter()
            .map(Value::try_from)
            .collect::<Result<Vec<_>, _>>()?
            .into())
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<Array> for Vec<serde_json::Value> {
    type Error = crate::InvalidUnicodeString;

    fn try_from(value: Array) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(serde_json::Value::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::number::Number;

    #[test]
    fn test_empty() {
        assert_eq!(Array::from_json(b"[]"), Ok(Array::new()));
    }

    #[test]
    fn test_one_number() {
        assert_eq!(
            Array::from_json(b"[1.2]"),
            Ok(vec![Value::Number(Number::try_from(1.2).unwrap())].into())
        );
    }

    #[test]
    fn test_mixed() {
        assert_eq!(
            Array::from_json(br#"[null, false, true, 1.2, "a,b", [""]]"#),
            Ok(vec![
                Value::Null,
                Value::Bool(false),
                Value::Bool(true),
                Number::try_from(1.2).unwrap().into(),
                "a,b".into(),
                vec!["".into()].into()
            ]
            .into())
        );
    }
}
