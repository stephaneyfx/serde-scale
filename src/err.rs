// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};
use core::fmt::{self, Debug, Display};

/// Serialization errors
#[derive(Debug)]
pub enum Error<E> {
    /// SCALE does not specify how to serialize floating point values
    FloatingPointUnsupported,
    /// SCALE limits enums to 255 variants
    TooManyVariants {
        enum_name: &'static str,
        variant_name: &'static str,
        variant_index: u32,
    },
    /// SCALE requires knowing the length of collections
    LengthNeeded,
    /// SCALE requires knowing the type of the data being deserialized
    TypeMustBeKnown,
    /// A boolean value (0 or 1) was expected but another byte was found
    ExpectedBoolean {
        found: u8,
    },
    /// Invalid character found. Characters must be UTF-32 code points.
    InvalidCharacter {
        found: u32,
    },
    /// This implementation limits collections to 2^64 elements
    CollectionTooLargeToSerialize {
        len: usize,
    },
    /// This implementation limits collections to 2^64 elements
    CollectionTooLargeToDeserialize,
    /// Invalid Unicode was found in a string
    InvalidUnicode(core::str::Utf8Error),
    /// An option was expected but the discriminant is invalid
    InvalidOption {
        found_discriminant: u8,
    },
    /// I/O error from the underlying reader or writer
    Io(E),
    /// Other error the serializer or deserializer might encounter
    #[cfg(feature = "alloc")]
    Other(String),
    #[cfg(not(feature = "alloc"))]
    Other(&'static str),
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Io(e)
    }
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FloatingPointUnsupported => {
                write!(f, "Floating point values are not supported by the SCALE encoding")
            }
            Error::TooManyVariants { enum_name, variant_name, variant_index } => {
                write!(f, "Variant {}::{} has index {} but the SCALE encoding limits enumerations \
                    to 255 variants", enum_name, variant_name, variant_index)
            }
            Error::LengthNeeded => {
                write!(f, "Sequence length unknown but the SCALE encoding requires to know it")
            }
            Error::TypeMustBeKnown => {
                write!(f, "Type unknown but the SCALE encoding requires to know it")
            }
            Error::ExpectedBoolean { found } => {
                write!(f, "Expected boolean (0 or 1), found {}", found)
            }
            Error::InvalidCharacter { found } => {
                write!(f, "{} is an invalid UTF-32 codepoint", found)
            }
            Error::CollectionTooLargeToSerialize { len } => {
                write!(f, "Found a collection of {} elements but this implementation limits \
                    collections to 2^64 elements", len)
            }
            Error::CollectionTooLargeToDeserialize => {
                write!(f, "Collections of more than 2^64 elements are not supported")
            }
            Error::InvalidUnicode(e) => {
                write!(f, "Invalid Unicode in string: {}", e)
            }
            Error::InvalidOption { found_discriminant } => {
                write!(f, "Invalid option. Expected a discriminant of 0 or 1 but found {}",
                    found_discriminant)
            }
            Error::Io(e) => {
                write!(f, "I/O error: {}", e)
            }
            Error::Other(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidUnicode(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::FloatingPointUnsupported
            | Error::TooManyVariants { .. }
            | Error::LengthNeeded
            | Error::TypeMustBeKnown
            | Error::ExpectedBoolean { .. }
            | Error::InvalidCharacter { .. }
            | Error::CollectionTooLargeToSerialize { .. }
            | Error::CollectionTooLargeToDeserialize
            | Error::InvalidOption { .. }
            | Error::Other(_) => None,
        }
    }
}

impl<E: SuperError + 'static> serde::ser::Error for Error<E> {
    fn custom<T: Display>(msg: T) -> Self {
        #[cfg(feature = "alloc")]
        {
            Error::Other(msg.to_string())
        }
        #[cfg(not(feature = "alloc"))]
        {
            let _ = msg;
            Error::Other("Custom error")
        }
    }
}

impl<E: SuperError + 'static> serde::de::Error for Error<E> {
    fn custom<T: Display>(msg: T) -> Self {
        serde::ser::Error::custom(msg)
    }
}

pub use super_error::SuperError;

#[cfg(feature = "std")]
mod super_error {
    pub use std::error::Error as SuperError;
}

#[cfg(not(feature = "std"))]
mod super_error {
    use core::fmt::{Debug, Display};

    pub trait SuperError: Debug + Display {}
    impl<E: Debug + Display + ?Sized> SuperError for E {}
}
