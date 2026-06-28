pub mod decoders;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

/// Represents a BinaryCIF encoded data column.
///
/// Encoded data consists of a byte buffer `data` and an array of `encoding` steps 
/// that describe how to decode the buffer back into its original types (e.g. Strings, Floats).
#[derive(Debug, Serialize, Deserialize)]
pub struct EncodedData {
    pub encoding: Vec<Encoding>,
    pub data: ByteBuf,
}

/// Describes a single step in a BinaryCIF decoding chain.
///
/// BinaryCIF uses a pipeline of encodings to compress numerical and string data.
/// To read the original data, apply the decoders in reverse order of the encoding chain.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Encoding {
    /// Reconstructs an array of strings from a concatenated string buffer and string offsets.
    #[serde(rename = "StringArray")]
    StringArray {
        #[serde(rename = "dataEncoding")]
        data_encoding: Vec<Encoding>,
        #[serde(rename = "stringData")]
        string_data: String,
        #[serde(rename = "offsetEncoding")]
        offset_encoding: Vec<Encoding>,
        offsets: ByteBuf,
    },
    /// Represents raw byte data, usually the last step in the decoding chain before interpreting bytes as basic types.
    #[serde(rename = "ByteArray")]
    ByteArray {
        #[serde(rename = "type")]
        data_type: i32,
    },
    /// Unpacks integers that were packed into 1, 2, or 3 bytes (depending on `byteCount`) back to 32-bit integers.
    #[serde(rename = "IntegerPacking")]
    IntegerPacking {
        #[serde(rename = "byteCount")]
        byte_count: i32,
        #[serde(rename = "isUnsigned")]
        is_unsigned: bool,
        #[serde(rename = "srcSize")]
        src_size: i32,
    },
    /// Reconstructs original values by applying a cumulative sum over delta-encoded differences.
    #[serde(rename = "Delta")]
    Delta {
        #[serde(rename = "origin")]
        origin: i32,
        #[serde(rename = "srcType")]
        src_type: i32,
    },
    /// Reconstructs an array by repeating a value `srcSize` times. 
    /// Often follows other encodings to reconstruct long runs of identical values.
    #[serde(rename = "RunLength")]
    RunLength {
        #[serde(rename = "srcSize")]
        src_size: i32,
        #[serde(rename = "srcType")]
        src_type: i32,
    },
    /// Recovers floating point values from integers by dividing them by `factor`.
    #[serde(rename = "FixedPoint")]
    FixedPoint {
        factor: f64,
        #[serde(rename = "srcType")]
        src_type: i32,
    },
    /// Recovers floating point values from quantized bins in the range `[min, max]` across `numSteps`.
    #[serde(rename = "IntervalQuantization")]
    IntervalQuantization {
        min: f64,
        max: f64,
        #[serde(rename = "numSteps")]
        num_steps: i32,
        #[serde(rename = "srcType")]
        src_type: i32,
    },
}

#[allow(dead_code)]
pub enum DataType {
    Int8 = 1,
    Int16 = 2,
    Int32 = 3,
    Uint8 = 4,
    Uint16 = 5,
    Uint32 = 6,
    Float32 = 32,
    Float64 = 33,
}
