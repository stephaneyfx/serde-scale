// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

use core::{
    fmt::{self, Debug, Display},
    ops::Deref,
};

/// Interface to read bytes
pub trait Read<'a> {
    type Error: Debug + Display;

    /// Reads exactly `n` bytes and passes them to the given function
    ///
    /// An error must be returned if there are fewer than `n` bytes left.
    fn read_map<R, F>(&mut self, n: usize, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(Bytes<'a, '_>) -> R;

    /// Reads exactly `buf.len()` bytes and writes them to the supplied buffer
    ///
    /// An error must be returned if there are fewer than `buf.len()` bytes left.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_map(buf.len(), |bytes| {
            buf.copy_from_slice(&bytes);
        })
    }
}

impl<'a, T: Read<'a> + ?Sized> Read<'a> for &'_ mut T {
    type Error = T::Error;

    fn read_map<R, F>(&mut self, n: usize, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(Bytes<'a, '_>) -> R,
    {
        (**self).read_map(n, f)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        (**self).read_exact(buf)
    }
}

impl<'a> Read<'a> for &'a [u8] {
    type Error = EndOfInput;

    fn read_map<R, F>(&mut self, n: usize, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(Bytes<'a, '_>) -> R,
    {
        if n > self.len() {
            return Err(EndOfInput);
        }
        let (consumed, remaining) = self.split_at(n);
        *self = remaining;
        Ok(f(Bytes::Persistent(consumed)))
    }
}

/// Bytes borrowed from the deserializer or valid only for the duration of the call to `read_map`
pub enum Bytes<'a, 'b> {
    /// Bytes borrowed from the deserializer allowing zero-copy deserialization
    Persistent(&'a [u8]),
    /// Bytes only valid for the duration of the call to `read_map`
    Temporary(&'b [u8]),
}

impl Deref for Bytes<'_, '_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            Bytes::Persistent(b) => b,
            Bytes::Temporary(b) => b,
        }
    }
}

/// Error indicating that the end of the input was reached and not enough bytes were read
#[derive(Debug)]
pub struct EndOfInput;

impl Display for EndOfInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EndOfInput")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EndOfInput {}
