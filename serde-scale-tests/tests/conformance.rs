// Copyright (C) 2020 Stephane Raux. Distributed under the zlib license.

use parity_scale_codec::{Encode, OptionBool};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    error::Error,
    fmt::Debug,
};

fn roundtrips<T>(v: &T) -> Result<(), Box<dyn Error>>
where
    T: Debug + Serialize + DeserializeOwned + PartialEq,
{
    let rebuilt = serde_scale::from_slice::<T>(&serde_scale::to_vec(v)?)?;
    if *v == rebuilt {
        Ok(())
    } else {
        let msg = format!(
            "Values before and after serialization differ.\n\
            Before: {:?}\n\
            After: {:?}",
            v,
            rebuilt,
        );
        Err(msg.into())
    }
}

fn same_as_codec<T, U>(v: &T, codec_v: &U) -> Result<(), Box<dyn Error>>
where
    T: Debug + Serialize,
    U: Encode,
{
    let out = serde_scale::to_vec(v)?;
    let codec_out = codec_v.encode();
    if out == codec_out {
        Ok(())
    } else {
        let msg = format!(
            "Serialization result differs from the reference implementation.\n\
            serde-scale: {:?}\n\
            parity-scale-codec: {:?}",
            out,
            codec_out,
        );
        Err(msg.into())
    }
}

struct TestInfo<T, U> {
    value: T,
    codec_value: U,
}

impl<T, U> TestInfo<T, U> {
    fn new(value: T, codec_value: U) -> Self {
        Self { value, codec_value }
    }
}

impl<T: Clone> From<T> for TestInfo<T, T> {
    fn from(x: T) -> Self {
        TestInfo {
            value: x.clone(),
            codec_value: x,
        }
    }
}

trait Test {
    fn run<T, U, I>(&self, info: I) -> Result<(), Box<dyn Error>>
    where
        I: Into<TestInfo<T, U>>,
        T: Debug + Serialize + DeserializeOwned + PartialEq,
        U: Encode;

    fn run_with<T, U>(&self, value: T, codec_value: U) -> Result<(), Box<dyn Error>>
    where
        T: Debug + Serialize + DeserializeOwned + PartialEq,
        U: Encode,
    {
        self.run(TestInfo::new(value, codec_value))
    }
}

struct Roundtrips;

impl Test for Roundtrips {
    fn run<T, U, I>(&self, info: I) -> Result<(), Box<dyn Error>>
    where
        I: Into<TestInfo<T, U>>,
        T: Debug + Serialize + DeserializeOwned + PartialEq,
        U: Encode,
    {
        let info = info.into();
        roundtrips(&info.value).map_err(|e| {
            format!("{:?} did not roundtrip:\n{}", info.value, e).into()
        })
    }
}

struct SameAsCodec;

impl Test for SameAsCodec {
    fn run<T, U, I>(&self, info: I) -> Result<(), Box<dyn Error>>
    where
        I: Into<TestInfo<T, U>>,
        T: Debug + Serialize + DeserializeOwned + PartialEq,
        U: Encode,
    {
        let info = info.into();
        same_as_codec(&info.value, &info.codec_value).map_err(|e| {
            format!(
                "{:?} serialized differently from reference implementation:\n{}", info.value, e,
            ).into()
        })
    }
}

fn apply_test<T: Test>(test: T) {
    let results = vec![
        test.run(i8::min_value()),
        test.run(1_i8),
        test.run(i8::max_value()),
        test.run(i16::min_value()),
        test.run(1_i8),
        test.run(i16::max_value()),
        test.run(i32::min_value()),
        test.run(1_i32),
        test.run(i32::max_value()),
        test.run(i64::min_value()),
        test.run(1_i64),
        test.run(i64::max_value()),
        test.run(u8::min_value()),
        test.run(1_u8),
        test.run(u8::max_value()),
        test.run(u16::min_value()),
        test.run(1_u16),
        test.run(u16::max_value()),
        test.run(u32::min_value()),
        test.run(1_u32),
        test.run(u32::max_value()),
        test.run(u64::min_value()),
        test.run(1_u64),
        test.run(u64::max_value()),
        test.run(false),
        test.run(true),
        test.run(None::<i32>),
        test.run(Some(3_i32)),
        test.run_with(None::<bool>, OptionBool(None)),
        test.run_with(Some(false), OptionBool(Some(false))),
        test.run_with(Some(true), OptionBool(Some(true))),
        test.run(Ok::<i32, String>(3)),
        test.run(Err::<String, i32>(3)),
        test.run(vec![1, 2, 3]),
        test.run(String::from("foo")),
        test.run((3, String::from("foo"))),
        test.run(Operator { name: "+".into(), priority: 2 }),
        test.run(Expression::Const(3)),
        test.run(Expression::Op(
            Box::new(Expression::Const(2)),
            Operator { name: "+".into(), priority: 2 },
            Box::new(Expression::Const(3)),
        )),
    ];
    let error_msg = results
        .into_iter()
        .flat_map(|r| r.err())
        .map(|e| format!("\n{}\n", e))
        .collect::<String>();
    assert!(error_msg.is_empty(), error_msg);
}

#[test]
fn data_set_roundtrips() {
    apply_test(Roundtrips);
}

#[test]
fn results_match_codec() {
    apply_test(SameAsCodec);
}

#[derive(Clone, Debug, Deserialize, Encode, PartialEq, Serialize)]
struct Operator {
    name: String,
    priority: u8,
}

#[derive(Clone, Debug, Deserialize, Encode, PartialEq, Serialize)]
enum Expression {
    Const(i32),
    Op(Box<Expression>, Operator, Box<Expression>),
}
