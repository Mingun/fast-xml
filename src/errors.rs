//! Error management module

use crate::escape::EscapeError;
use crate::events::attributes::AttrError;
use std::str::Utf8Error;

/// The error type used by this crate.
#[derive(Debug)]
pub enum Error {
    /// IO error
    Io(::std::io::Error),
    /// Utf8 error
    Utf8(Utf8Error),
    /// Unexpected End of File
    UnexpectedEof(String),
    /// End event mismatch
    EndEventMismatch {
        /// Expected end event
        expected: String,
        /// Found end event
        found: String,
    },
    /// Unexpected token
    UnexpectedToken(String),
    /// Unexpected <!>
    UnexpectedBang(u8),
    /// Text not found, expected `Event::Text`
    TextNotFound,
    /// `Event::XmlDecl` must start with *version* attribute
    XmlDeclWithoutVersion(Option<String>),
    /// Attribute parsing error
    InvalidAttr(AttrError),
    /// Escape error
    EscapeError(EscapeError),
}

impl From<::std::io::Error> for Error {
    /// Creates a new `Error::Io` from the given error
    #[inline]
    fn from(error: ::std::io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<Utf8Error> for Error {
    /// Creates a new `Error::Utf8` from the given error
    #[inline]
    fn from(error: Utf8Error) -> Error {
        Error::Utf8(error)
    }
}

impl From<EscapeError> for Error {
    /// Creates a new `Error::EscapeError` from the given error
    #[inline]
    fn from(error: EscapeError) -> Error {
        Error::EscapeError(error)
    }
}

impl From<AttrError> for Error {
    #[inline]
    fn from(error: AttrError) -> Self {
        Error::InvalidAttr(error)
    }
}

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Utf8(e) => write!(f, "UTF8 error: {}", e),
            Error::UnexpectedEof(e) => write!(f, "Unexpected EOF during reading {}", e),
            Error::EndEventMismatch { expected, found } => {
                write!(f, "Expecting </{}> found </{}>", expected, found)
            }
            Error::UnexpectedToken(e) => write!(f, "Unexpected token '{}'", e),
            Error::UnexpectedBang(b) => write!(
                f,
                "Only Comment (`--`), CDATA (`[CDATA[`) and DOCTYPE (`DOCTYPE`) nodes can start with a '!', but symbol `{}` found",
                *b as char
            ),
            Error::TextNotFound => write!(f, "Cannot read text, expecting Event::Text"),
            Error::XmlDeclWithoutVersion(e) => write!(
                f,
                "XmlDecl must start with 'version' attribute, found {:?}",
                e
            ),
            Error::InvalidAttr(e) => write!(f, "error while parsing attribute: {}", e),
            Error::EscapeError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            Error::InvalidAttr(e) => Some(e),
            Error::EscapeError(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "serialize")]
pub mod serialize {
    //! A module to handle serde (de)serialization errors

    use super::*;
    use crate::utils::write_byte_string;
    use std::fmt;
    #[cfg(feature = "overlapped-lists")]
    use std::num::NonZeroUsize;
    use std::num::{ParseFloatError, ParseIntError};

    /// (De)serialization error
    #[derive(Debug)]
    pub enum DeError {
        /// Serde custom error
        Custom(String),
        /// Xml parsing error
        InvalidXml(Error),
        /// Cannot parse to integer
        InvalidInt(ParseIntError),
        /// Cannot parse to float
        InvalidFloat(ParseFloatError),
        /// Cannot parse specified value to boolean
        InvalidBoolean(String),
        /// This error indicates an error in the [`Deserialize`](serde::Deserialize)
        /// implementation when read a map or a struct: `MapAccess::next_value[_seed]`
        /// was called before `MapAccess::next_key[_seed]`.
        ///
        /// You should check your types, that implements corresponding trait.
        KeyNotRead,
        /// Deserializer encounter a start tag with a specified name when it is
        /// not expecting. This happens when you try to deserialize a primitive
        /// value (numbers, strings, booleans) from an XML element.
        UnexpectedStart(Vec<u8>),
        /// Deserializer encounter an end tag with a specified name when it is
        /// not expecting. Usually that should not be possible, because XML reader
        /// is not able to produce such stream of events that lead to this error.
        ///
        /// If you get this error this likely indicates and error in the `fast_xml`.
        /// Please open an issue at <https://github.com/Mingun/fast-xml>, provide
        /// your Rust code and XML input.
        UnexpectedEnd(Vec<u8>),
        /// Unexpected end of file
        UnexpectedEof,
        /// This error indicates that [`deserialize_struct`] was called, but there
        /// is no any XML element in the input. That means that you try to deserialize
        /// a struct not from an XML element.
        ///
        /// [`deserialize_struct`]: serde::de::Deserializer::deserialize_struct
        ExpectedStart,
        /// Unsupported operation
        Unsupported(&'static str),
        /// Too many events was skipped during deserializing a sequence, limit of
        /// events exceeded. The limit provides as an argument
        #[cfg(feature = "overlapped-lists")]
        TooManyEvents(NonZeroUsize),
    }

    impl fmt::Display for DeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                DeError::Custom(s) => write!(f, "{}", s),
                DeError::InvalidXml(e) => write!(f, "{}", e),
                DeError::InvalidInt(e) => write!(f, "{}", e),
                DeError::InvalidFloat(e) => write!(f, "{}", e),
                DeError::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
                DeError::KeyNotRead => write!(f, "Invalid `Deserialize` implementation: `MapAccess::next_value[_seed]` was called before `MapAccess::next_key[_seed]`"),
                DeError::UnexpectedStart(e) => {
                    f.write_str("Unexpected `Event::Start(")?;
                    write_byte_string(f, &e)?;
                    f.write_str(")`")
                }
                DeError::UnexpectedEnd(e) => {
                    f.write_str("Unexpected `Event::End(")?;
                    write_byte_string(f, &e)?;
                    f.write_str(")`")
                }
                DeError::UnexpectedEof => write!(f, "Unexpected `Event::Eof`"),
                DeError::ExpectedStart => write!(f, "Expecting `Event::Start`"),
                DeError::Unsupported(s) => write!(f, "Unsupported operation {}", s),
                #[cfg(feature = "overlapped-lists")]
                DeError::TooManyEvents(s) => write!(f, "Deserializer buffers {} events, limit exceeded", s),
            }
        }
    }

    impl ::std::error::Error for DeError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                DeError::InvalidXml(e) => Some(e),
                DeError::InvalidInt(e) => Some(e),
                DeError::InvalidFloat(e) => Some(e),
                _ => None,
            }
        }
    }

    impl serde::de::Error for DeError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            DeError::Custom(msg.to_string())
        }
    }

    impl serde::ser::Error for DeError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            DeError::Custom(msg.to_string())
        }
    }

    impl From<Error> for DeError {
        fn from(e: Error) -> Self {
            Self::InvalidXml(e)
        }
    }

    impl From<EscapeError> for DeError {
        #[inline]
        fn from(e: EscapeError) -> Self {
            Self::InvalidXml(e.into())
        }
    }

    impl From<ParseIntError> for DeError {
        fn from(e: ParseIntError) -> Self {
            Self::InvalidInt(e)
        }
    }

    impl From<ParseFloatError> for DeError {
        fn from(e: ParseFloatError) -> Self {
            Self::InvalidFloat(e)
        }
    }

    impl From<AttrError> for DeError {
        #[inline]
        fn from(e: AttrError) -> Self {
            Self::InvalidXml(e.into())
        }
    }
}
