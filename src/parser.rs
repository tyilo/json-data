use crate::error::Error;

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn read_all<T>(
        bytes: &'a [u8],
        f: impl FnOnce(&mut Reader) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut parser = Reader::new(bytes);
        let v = f(&mut parser)?;
        if !parser.at_end() {
            return Err(Error::TrailingData);
        }
        Ok(v)
    }

    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn at_end(&self) -> bool {
        self.bytes.is_empty()
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8, Error> {
        match self.bytes.split_first() {
            Some((b, rest)) => {
                self.bytes = rest;
                Ok(*b)
            }
            None => Err(Error::UnexpectedEof),
        }
    }

    pub(crate) fn peek_byte(&self) -> Option<u8> {
        self.bytes.first().copied()
    }

    pub(crate) fn read_bytes<const N: usize>(&mut self) -> Result<&'a [u8; N], Error> {
        match self.bytes.split_first_chunk() {
            Some((chunk, rest)) => {
                self.bytes = rest;
                Ok(chunk)
            }
            None => Err(Error::UnexpectedEof),
        }
    }

    pub(crate) fn read_char(&mut self) -> Result<char, Error> {
        let remaining = self.bytes.len();
        if remaining == 0 {
            return Err(Error::UnexpectedEof);
        }
        for n in 1..=remaining.min(4) {
            if let Ok(str) = std::str::from_utf8(&self.bytes[..n]) {
                let mut chars = str.chars();
                let char = chars.next().unwrap();
                assert_eq!(chars.next(), None);
                self.bytes = self.bytes.split_at(n).1;
                return Ok(char);
            }
        }
        Err(Error::InvalidUtf8Char)
    }

    pub(crate) fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek_byte() {
            match b {
                b'\t' | b'\n' | b'\r' | b' ' => {}
                _ => break,
            }
            self.read_byte().unwrap();
        }
    }

    pub(crate) fn parse_slice<T>(
        &mut self,
        f: impl FnOnce(&mut Reader) -> Result<T, Error>,
    ) -> Result<(&'a [u8], T), Error> {
        let bytes_start = self.bytes;
        let v = f(self)?;

        let start = bytes_start.as_ptr();
        let end = self.bytes.as_ptr();
        let slice = unsafe {
            std::slice::from_raw_parts(start, end.offset_from(start).try_into().unwrap())
        };

        Ok((slice, v))
    }
}
