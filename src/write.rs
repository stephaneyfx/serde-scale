// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use crate::SuperError;

/// Interface to write bytes
pub trait Write {
    type Error: SuperError + 'static;

    /// Writes bytes
    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
}

impl<W: Write + ?Sized> Write for &'_ mut W {
    type Error = W::Error;

    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        (**self).write(data)
    }
}

#[cfg(feature = "alloc")]
impl Write for Vec<u8> {
    type Error = core::convert::Infallible;

    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.extend(data);
        Ok(())
    }
}
