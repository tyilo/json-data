use std::{collections::BTreeMap, fmt::Display};

use crate::{
    error::Error,
    parser::Reader,
    read_value,
    string::{read_string, JsonString},
    Value,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Object {
    inner: BTreeMap<JsonString, Value>,
}

impl Object {
    pub fn from_json(bytes: &[u8]) -> Result<Object, Error> {
        Reader::read_all(bytes, read_object)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

pub(crate) fn read_object(reader: &mut Reader) -> Result<Object, Error> {
    match reader.read_byte()? {
        b'{' => {}
        b => {
            return Err(Error::ExpectedLeftBrace(b));
        }
    }
    reader.skip_whitespace();
    if reader.peek_byte() == Some(b'}') {
        reader.read_byte()?;
        return Ok(Object::default());
    }
    let mut inner = BTreeMap::new();
    loop {
        let key = read_string(reader)?;

        reader.skip_whitespace();

        match reader.read_byte()? {
            b':' => {}
            b => return Err(Error::ExpectedColon(b)),
        }

        let value = read_value(reader)?;

        inner.insert(key, value);

        reader.skip_whitespace();
        match reader.read_byte()? {
            b',' => {}
            b'}' => break,
            b => return Err(Error::ExpectedCommaOrRightBrace(b)),
        }

        reader.skip_whitespace();
    }
    Ok(Object { inner })
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.inner.iter().enumerate() {
            if i != 0 {
                write!(f, ",")?;
            }
            write!(f, "{k}:{v}")?;
        }
        write!(f, "}}")
    }
}

impl From<BTreeMap<JsonString, Value>> for Object {
    fn from(value: BTreeMap<JsonString, Value>) -> Self {
        Self { inner: value }
    }
}

impl IntoIterator for Object {
    type Item = (JsonString, Value);
    type IntoIter = <BTreeMap<JsonString, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<serde_json::Map<String, serde_json::Value>> for Object {
    type Error = crate::InvalidSerdeJsonNumber;

    fn try_from(value: serde_json::Map<String, serde_json::Value>) -> Result<Self, Self::Error> {
        Ok(value
            .into_iter()
            .map(|(k, v)| v.try_into().map(|v| (k.into(), v)))
            .collect::<Result<BTreeMap<_, _>, _>>()?
            .into())
    }
}

#[cfg(feature = "serde_json")]
impl TryFrom<Object> for serde_json::Map<String, serde_json::Value> {
    type Error = crate::InvalidUnicodeString;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(|(k, v)| {
                Ok((
                    k.into_string().map_err(crate::InvalidUnicodeString)?,
                    v.try_into()?,
                ))
            })
            .collect::<Result<serde_json::Map<String, serde_json::Value>, _>>()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_obj_with_whitespace() {
        let o = Object::from_json(br#"{ "a" : "b" , "c" : "d" }"#).unwrap();
        assert_eq!(o.inner.len(), 2);
        assert_eq!(o.inner.get(&"a".into()), Some(&"b".into()));
        assert_eq!(o.inner.get(&"c".into()), Some(&"d".into()));
    }
}
