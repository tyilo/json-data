use std::{
    collections::{btree_map, BTreeMap},
    fmt::Display,
};

use crate::{
    error::Error,
    parser::Reader,
    read_value,
    string::{read_string, JsonStr, JsonString},
    Value,
};

type Map = BTreeMap<JsonString, Value>;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Object {
    inner: Map,
}

impl Object {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json(bytes: &[u8]) -> Result<Object, Error> {
        Reader::read_all(bytes, read_object)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn into_inner(self) -> Map {
        self.inner
    }

    pub fn as_inner(&self) -> &Map {
        &self.inner
    }

    pub fn as_inner_mut(&mut self) -> &mut Map {
        &mut self.inner
    }

    pub fn get<'a, Q>(&self, key: &'a Q) -> Option<&Value>
    where
        Q: ?Sized,
        &'a Q: Into<&'a JsonStr>,
    {
        self.inner.get(key.into())
    }

    pub fn get_mut<'a, Q>(&mut self, key: &'a Q) -> Option<&mut Value>
    where
        Q: ?Sized,
        &'a Q: Into<&'a JsonStr>,
    {
        self.inner.get_mut(key.into())
    }

    pub fn contains_key<'a, Q>(&self, key: &'a Q) -> bool
    where
        Q: ?Sized,
        &'a Q: Into<&'a JsonStr>,
    {
        self.inner.contains_key(key.into())
    }

    pub fn remove<'a, Q>(&mut self, key: &'a Q) -> Option<Value>
    where
        Q: ?Sized,
        &'a Q: Into<&'a JsonStr>,
    {
        self.inner.remove(key.into())
    }

    pub fn insert(&mut self, key: JsonString, value: Value) -> Option<Value> {
        self.inner.insert(key, value)
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn entry(&mut self, key: JsonString) -> Entry<'_> {
        match self.inner.entry(key) {
            btree_map::Entry::Vacant(entry) => Entry::Vacant(VacantEntry(entry)),
            btree_map::Entry::Occupied(entry) => Entry::Occupied(OccupiedEntry(entry)),
        }
    }
}

pub enum Entry<'a> {
    Vacant(VacantEntry<'a>),
    Occupied(OccupiedEntry<'a>),
}

impl<'a> Entry<'a> {
    fn from_inner_entry(entry: btree_map::Entry<'a, JsonString, Value>) -> Self {
        match entry {
            btree_map::Entry::Vacant(entry) => Entry::Vacant(VacantEntry(entry)),
            btree_map::Entry::Occupied(entry) => Entry::Occupied(OccupiedEntry(entry)),
        }
    }

    fn into_inner_entry(self) -> btree_map::Entry<'a, JsonString, Value> {
        match self {
            Entry::Vacant(entry) => btree_map::Entry::Vacant(entry.0),
            Entry::Occupied(entry) => btree_map::Entry::Occupied(entry.0),
        }
    }

    pub fn or_insert(self, default: Value) -> &'a mut Value {
        self.into_inner_entry().or_insert(default)
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut Value
    where
        F: FnOnce() -> Value,
    {
        self.into_inner_entry().or_insert_with(default)
    }

    pub fn or_insert_with_key<F>(self, default: F) -> &'a mut Value
    where
        F: FnOnce(&JsonString) -> Value,
    {
        self.into_inner_entry().or_insert_with_key(default)
    }

    pub fn key(&self) -> &JsonString {
        match self {
            Entry::Vacant(entry) => entry.key(),
            Entry::Occupied(entry) => entry.key(),
        }
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Value),
    {
        Self::from_inner_entry(self.into_inner_entry().and_modify(f))
    }
}

pub struct VacantEntry<'a>(btree_map::VacantEntry<'a, JsonString, Value>);

impl<'a> VacantEntry<'a> {
    pub fn key(&self) -> &JsonString {
        self.0.key()
    }

    pub fn into_key(self) -> JsonString {
        self.0.into_key()
    }

    pub fn insert(self, value: Value) -> &'a mut Value {
        self.0.insert(value)
    }
}

pub struct OccupiedEntry<'a>(btree_map::OccupiedEntry<'a, JsonString, Value>);

impl<'a> OccupiedEntry<'a> {
    pub fn key(&self) -> &JsonString {
        self.0.key()
    }

    pub fn remove_entry(self) -> (JsonString, Value) {
        self.0.remove_entry()
    }

    pub fn get(&self) -> &Value {
        self.0.get()
    }

    pub fn get_mut(&mut self) -> &mut Value {
        self.0.get_mut()
    }

    pub fn into_mut(self) -> &'a mut Value {
        self.0.into_mut()
    }

    pub fn insert(&mut self, value: Value) -> Value {
        self.0.insert(value)
    }

    pub fn remove(self) -> Value {
        self.0.remove()
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
    let mut inner = Map::new();
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

impl From<Map> for Object {
    fn from(value: Map) -> Self {
        Self { inner: value }
    }
}

impl IntoIterator for Object {
    type Item = (JsonString, Value);
    type IntoIter = <Map as IntoIterator>::IntoIter;

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
            .collect::<Result<Map, _>>()?
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
        assert_eq!(o.len(), 2);
        assert_eq!(o.get("a"), Some(&"b".into()));
        assert_eq!(o.get("c"), Some(&"d".into()));
    }

    #[test]
    fn test_object() {
        let mut obj = Object::new();
        obj.insert(JsonString::from("abc"), Value::Null);
        assert_eq!(obj.get("abc"), Some(&Value::Null));
    }
}
