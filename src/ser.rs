// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

use crate::{Error, Write};
use serde::Serialize;
use core::{
    convert::TryFrom,
    fmt::{self, Debug, Display},
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Serializes a value using the SCALE encoding
#[cfg(feature = "alloc")]
pub fn to_vec<T: Serialize>(x: &T) -> Result<Vec<u8>, Error<core::convert::Infallible>> {
    let mut serializer = Serializer::new(Vec::new());
    x.serialize(&mut serializer)?;
    Ok(serializer.0)
}

/// Serializer for the SCALE encoding
#[derive(Debug)]
pub struct Serializer<W>(W);

impl<W: Write> Serializer<W> {
    /// Returns a serializer using the given writer
    pub fn new(out: W) -> Self {
        Self(out)
    }

    /// Returns the underlying writer
    pub fn into_inner(self) -> W {
        self.0
    }

    fn serialize_compact(&mut self, v: u64) -> Result<(), Error<W::Error>> {
        if v < 0x40 {
            let bytes = [(v << 2 & 0xff) as u8];
            Ok(self.0.write(&bytes)?)
        } else if v < 0x4000 {
            let bytes = [
                ((v << 2 | 0x1) & 0xff) as u8,
                (v >> 6 & 0xff) as u8,
            ];
            Ok(self.0.write(&bytes)?)
        } else if v < 0x4000_0000 {
            let high = v >> 6;
            let bytes = [
                ((v << 2 | 0x2) & 0xff) as u8,
                (high & 0xff) as u8,
                (high >> 8 & 0xff) as u8,
                (high >> 16 & 0xff) as u8,
            ];
            Ok(self.0.write(&bytes)?)
        } else {
            let mut bytes = [0u8; 9];
            let mut v = v;
            let src = core::iter::from_fn(|| {
                if v == 0 { return None; }
                let low = (v & 0xff) as u8;
                v >>= 8;
                Some(low)
            });
            let end = bytes.iter_mut()
                .skip(1)
                .zip(src)
                .enumerate()
                .map(|(i, (dst, src))| {
                    *dst = src;
                    i
                })
                .last()
                .unwrap() + 1;
            bytes[0] = (end - 4 << 2 & 0x3) as u8;
            Ok(self.0.write(&bytes[..end + 1])?)
        }
    }
}

impl<'a, W: Write> serde::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error<W::Error>;
    type SerializeSeq = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeTupleVariant = Compound<'a, W>;
    type SerializeMap = Compound<'a, W>;
    type SerializeStruct = Compound<'a, W>;
    type SerializeStructVariant = Compound<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_u8(v as u8)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.write(&v.to_le_bytes())?)
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Err(Error::FloatingPointUnsupported)
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Err(Error::FloatingPointUnsupported)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let len = v.len();
        let len = u64::try_from(len).map_err(|_| Error::CollectionTooLargeToSerialize { len })?;
        self.serialize_compact(len)?;
        Ok(self.0.write(v)?)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_u8(0)
    }

    fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        if let Ok(x) = v.serialize(OptionalBoolSerializer) {
            return self.serialize_u8(x);
        }
        self.serialize_u8(1)?;
        v.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let discriminant = u8::try_from(variant_index).map_err(|_| {
            Error::TooManyVariants {
                enum_name: name,
                variant_name: variant,
                variant_index,
            }
        })?;
        self.serialize_u8(discriminant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.serialize_unit_variant(name, variant_index, variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let len = len.ok_or(Error::LengthNeeded)?;
        let len = u64::try_from(len).map_err(|_| Error::CollectionTooLargeToSerialize { len })?;
        self.serialize_compact(len)?;
        Ok(Compound(self))
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(Compound(self))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(Compound(self))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        Ok(Compound(self))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let len = len.ok_or(Error::LengthNeeded)?;
        let len = u64::try_from(len).map_err(|_| Error::CollectionTooLargeToSerialize { len })?;
        self.serialize_compact(len)?;
        Ok(Compound(self))
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Compound(self))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        Ok(Compound(self))
    }

    #[cfg(not(feature = "alloc"))]
    fn collect_str<T: ?Sized>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: core::fmt::Display,
    {
        Err(serde::ser::Error::custom("Unsupported `collect_str` without `alloc` feature"))
    }
}

mod compound {
    pub struct Compound<'a, W>(pub &'a mut super::Serializer<W>);
}

use compound::Compound;

impl<W: Write> serde::ser::SerializeSeq for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeTuple for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeTupleStruct for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeTupleVariant for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeMap for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.0.serialize_compact(2)?;
        key.serialize(&mut *self.0)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeStruct for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, _: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: Write> serde::ser::SerializeStructVariant for Compound<'_, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, _: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct OptionalBoolSerializer;
type Impossible = serde::ser::Impossible<u8, VoidError>;

#[derive(Debug)]
struct VoidError;

impl Display for VoidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VoidError")
    }
}

impl serde::ser::StdError for VoidError {}

impl serde::ser::Error for VoidError {
    fn custom<T: Display>(_: T) -> Self {
        VoidError
    }
}

impl serde::Serializer for OptionalBoolSerializer {
    type Ok = u8;
    type Error = VoidError;
    type SerializeSeq = Impossible;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;
    type SerializeMap = Impossible;
    type SerializeStruct = Impossible;
    type SerializeStructVariant = Impossible;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(if v { 1 } else { 2 })
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_some<T>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(VoidError)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(VoidError)
    }

    fn serialize_newtype_struct<T>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(VoidError)
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(VoidError)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(VoidError)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(VoidError)
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(VoidError)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(VoidError)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(VoidError)
    }

    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(VoidError)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(VoidError)
    }

    #[cfg(not(feature = "alloc"))]
    fn collect_str<T: ?Sized>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: core::fmt::Display,
    {
        Err(VoidError)
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use crate::to_vec;

    #[test]
    fn none_bool_serializes_as_0() {
        assert_eq!(to_vec(&None::<bool>).unwrap(), [0]);
    }

    #[test]
    fn some_true_serializes_as_1() {
        assert_eq!(to_vec(&Some(true)).unwrap(), [1]);
    }

    #[test]
    fn some_false_serializes_as_2() {
        assert_eq!(to_vec(&Some(false)).unwrap(), [2]);
    }
}
