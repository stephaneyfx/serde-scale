// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

use core::convert::TryFrom;
use crate::{Bytes, EndOfInput, Error, Read};
use serde::{
    de::{DeserializeSeed, Visitor},
    Deserialize, Deserializer as _,
};

/// Deserializes a value encoded with SCALE
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T, Error<EndOfInput>>
where
    T: Deserialize<'a>,
{
    T::deserialize(&mut Deserializer(v))
}

/// Deserializer for the SCALE encoding
pub struct Deserializer<R>(R);

impl<'de, R: Read<'de>> Deserializer<R> {
    /// Returns a deserializer using the given reader
    pub fn new(r: R) -> Self {
        Self(r)
    }

    /// Returns the underlying reader
    pub fn into_inner(self) -> R {
        self.0
    }

    fn read_compact(&mut self) -> Result<u64, Error<R::Error>> {
        let mut head = 0;
        self.0.read_exact(core::slice::from_mut(&mut head))?;
        match head & 0x3 {
            0x0 => Ok((head >> 2) as u64),
            0x1 => {
                let low = (head >> 2) as u64;
                let high = self.read_u8()? as u64;
                Ok(low | high << 6)
            }
            0x2 => {
                let low = (head >> 2) as u64;
                let mut high = [0; 4];
                self.0.read_exact(&mut high[..3])?;
                let high = u32::from_le_bytes(high) as u64;
                Ok(low | high << 6)
            }
            0x3 => {
                let len = (head >> 2) as usize + 4;
                if len > 8 {
                    return Err(Error::CollectionTooLargeToDeserialize);
                }
                let mut buf = [0; 8];
                self.0.read_exact(&mut buf[..len])?;
                let n = u64::from_le_bytes(buf);
                Ok(n)
            }
            _ => unreachable!(),
        }
    }

    fn read_u8(&mut self) -> Result<u8, Error<R::Error>> {
        let mut v = 0;
        self.0.read_exact(core::slice::from_mut(&mut v))?;
        Ok(v)
    }

    fn read_u32(&mut self) -> Result<u32, Error<R::Error>> {
        let mut v = [0; 4];
        self.0.read_exact(&mut v)?;
        Ok(u32::from_le_bytes(v))
    }
}

impl<'de, R: Read<'de>> serde::Deserializer<'de> for &mut Deserializer<R> {
    type Error = Error<R::Error>;

    fn deserialize_any<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::TypeMustBeKnown)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.read_u8()? {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            found => Err(Error::ExpectedBoolean { found }),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0];
        self.0.read_exact(&mut found)?;
        visitor.visit_i8(i8::from_le_bytes(found))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0; 2];
        self.0.read_exact(&mut found)?;
        visitor.visit_i16(i16::from_le_bytes(found))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0; 4];
        self.0.read_exact(&mut found)?;
        visitor.visit_i32(i32::from_le_bytes(found))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0; 8];
        self.0.read_exact(&mut found)?;
        visitor.visit_i64(i64::from_le_bytes(found))

    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.read_u8()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0; 2];
        self.0.read_exact(&mut found)?;
        visitor.visit_u16(u16::from_le_bytes(found))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.read_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut found = [0; 8];
        self.0.read_exact(&mut found)?;
        visitor.visit_u64(u64::from_le_bytes(found))
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::FloatingPointUnsupported)
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::FloatingPointUnsupported)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let found = self.read_u32()?;
        let c = core::char::from_u32(found).ok_or(Error::InvalidCharacter { found })?;
        visitor.visit_char(c)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let n = self.read_compact()?;
        let n = usize::try_from(n).map_err(|_| Error::CollectionTooLargeToDeserialize)?;
        self.0.read_map(n, |bytes| {
            match bytes {
                Bytes::Persistent(b) => {
                    let s = core::str::from_utf8(b).map_err(Error::InvalidUnicode)?;
                    visitor.visit_borrowed_str(s)
                }
                Bytes::Temporary(b) => {
                    let s = core::str::from_utf8(b).map_err(Error::InvalidUnicode)?;
                    visitor.visit_str(s)
                }
            }
        })?
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let n = self.read_compact()?;
        let n = usize::try_from(n).map_err(|_| Error::CollectionTooLargeToDeserialize)?;
        self.0.read_map(n, |bytes| {
            match bytes {
                Bytes::Persistent(b) => visitor.visit_borrowed_bytes(b),
                Bytes::Temporary(b) => visitor.visit_bytes(b),
            }
        })?
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.read_u8()? {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(OptionalBoolDeserializer::discriminant_1(self)),
            2 => visitor.visit_some(OptionalBoolDeserializer::discriminant_2(self)),
            found_discriminant => Err(Error::InvalidOption { found_discriminant }),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let len = self.read_compact()?;
        let len = usize::try_from(len).map_err(|_| Error::CollectionTooLargeToDeserialize)?;
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(Sequence {
            deserializer: self,
            remaining: len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let len = self.read_compact()?;
        let len = usize::try_from(len).map_err(|_| Error::CollectionTooLargeToDeserialize)?;
        visitor.visit_map(Map {
            deserializer: self,
            remaining: len,
        })
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(Enum {
            deserializer: self,
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.read_u8()?)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct Sequence<'a, R> {
    deserializer: &'a mut Deserializer<R>,
    remaining: usize,
}

impl<'a, 'de, R: Read<'de>> serde::de::SeqAccess<'de> for Sequence<'a, R> {
    type Error = Error<R::Error>;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.deserializer).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

struct Map<'a, R> {
    deserializer: &'a mut Deserializer<R>,
    remaining: usize,
}

impl<'a, 'de, R: Read<'de>> serde::de::MapAccess<'de> for Map<'a, R> {
    type Error = Error<R::Error>;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.deserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct Enum<'a, R> {
    deserializer: &'a mut Deserializer<R>,
}

impl<'a, 'de, R: Read<'de>> serde::de::EnumAccess<'de> for Enum<'a, R> {
    type Error = Error<R::Error>;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self.deserializer)?, self))
    }
}

impl<'a, 'de, R: Read<'de>> serde::de::VariantAccess<'de> for Enum<'a, R> {
    type Error = Error<R::Error>;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.deserializer)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserializer.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserializer.deserialize_seq(visitor)
    }
}

struct OptionalBoolDeserializer<'a, R> {
    inner: &'a mut Deserializer<R>,
    discriminant_is_1: bool,
}

impl<'a, 'de, R: Read<'de>> OptionalBoolDeserializer<'a, R> {
    fn discriminant_1(inner: &'a mut Deserializer<R>) -> Self {
        Self {
            inner,
            discriminant_is_1: true,
        }
    }

    fn discriminant_2(inner: &'a mut Deserializer<R>) -> Self {
        Self {
            inner,
            discriminant_is_1: false,
        }
    }

    fn check_bad_discriminant(&self) -> Result<(), Error<R::Error>> {
        if self.discriminant_is_1 {
            Ok(())
        } else {
            Err(Error::InvalidOption { found_discriminant: 2 })
        }
    }
}

impl<'de, R: Read<'de>> serde::Deserializer<'de> for OptionalBoolDeserializer<'_, R> {
    type Error = Error<R::Error>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.discriminant_is_1)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_i64(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_u64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_f64(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_char(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_option(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_seq(visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_enum(name, variants, visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.check_bad_discriminant()?;
        self.inner.deserialize_ignored_any(visitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::from_slice;

    #[test]
    fn none_bool_deserializes_from_0() {
        assert_eq!(from_slice::<Option<bool>>(&[0]).unwrap(), None);
    }

    #[test]
    fn some_true_deserializes_from_1() {
        assert_eq!(from_slice::<Option<bool>>(&[1]).unwrap(), Some(true));
    }

    #[test]
    fn some_false_deserializes_from_2() {
        assert_eq!(from_slice::<Option<bool>>(&[2]).unwrap(), Some(false));
    }
}
