pub mod parser;

use crate::encoding::EncodedData;
use serde::{Deserialize, Serialize};

/// Represents the root container of a BinaryCIF file.
///
/// A `File` contains version and encoder metadata, followed by a list of `DataBlock`s.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub version: String,
    pub encoder: String,
    pub data_blocks: Vec<DataBlock>,
}

/// A data block representing a collection of categories.
///
/// Equivalent to a `data_` block in a text CIF file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBlock {
    pub header: String,
    pub categories: Vec<Category>,
}

/// A category holding a set of tabular data columns.
///
/// Equivalent to a loop or category in a text CIF file (e.g. `_atom_site`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub name: String,
    pub row_count: u32,
    pub columns: Vec<Column>,
}

/// A single column of data within a category.
///
/// Holds the actual encoded values in `data` and an optional `mask` which indicates
/// the presence or absence of data for each row (e.g. `.` or `?` in text CIF).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub data: EncodedData,
    pub mask: Option<EncodedData>,
}
