#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedEof,
    TrailingData,
    InvalidControlCharacter(u8),
    ExpectedDoubleQuote(u8),
    UnexpectedEscape(u8),
    InvalidHexChar(u8),
    InvalidUtf8Char,
    UnexpectedStartOfValue(u8),
    ExpectedNull,
    ExpectedTrue,
    ExpectedFalse,
    InvalidDigit(u8),
    InfiniteFloat,

    ExpectedLeftBracket(u8),
    ExpectedCommaOrRightBracket(u8),

    ExpectedLeftBrace(u8),
    ExpectedColon(u8),
    ExpectedCommaOrRightBrace(u8),
}
