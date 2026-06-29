use crate::encoding::{EncodedData, Encoding};
use crate::streaming::{Category, Column, DataBlock, File as BcifFile};
use serde_bytes::ByteBuf;
use std::fs::File;
use std::io::BufWriter;

pub fn create_sample_bcif(path: &str) -> anyhow::Result<()> {
    let bcif = create_complex_sample();
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    rmp_serde::encode::write_named(&mut writer, &bcif)?;
    Ok(())
}

pub fn create_single_row_sample_bcif(path: &str) -> anyhow::Result<()> {
    let category = Category {
        name: "_single_row_category".to_string(),
        row_count: 1,
        columns: vec![Column {
            name: "id".to_string(),
            data: EncodedData {
                encoding: vec![Encoding::ByteArray { data_type: 1 }],
                data: ByteBuf::from(vec![42]),
            },
            mask: None,
        }],
    };
    let bcif = BcifFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        encoder: "test-single-row".to_string(),
        data_blocks: vec![DataBlock {
            header: "SINGLE_ROW_BLOCK".to_string(),
            categories: vec![category],
        }],
    };
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    rmp_serde::encode::write_named(&mut writer, &bcif)?;
    Ok(())
}

pub fn create_large_bcif(path: &str, num_categories: usize) -> anyhow::Result<()> {
    let mut categories = Vec::with_capacity(num_categories);
    for i in 0..num_categories {
        categories.push(Category {
            name: format!("_cat_{}", i),
            row_count: 100,
            columns: vec![Column {
                name: "data".to_string(),
                data: EncodedData {
                    encoding: vec![Encoding::ByteArray { data_type: 1 }],
                    data: ByteBuf::from(vec![42; 100]),
                },
                mask: None,
            }],
        });
    }

    let bcif = BcifFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        encoder: "large-test-gen".to_string(),
        data_blocks: vec![DataBlock {
            header: "LARGE_BLOCK".to_string(),
            categories,
        }],
    };

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    rmp_serde::encode::write_named(&mut writer, &bcif)?;
    Ok(())
}

pub fn create_complex_sample() -> BcifFile {
    BcifFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        encoder: "test-generator".to_string(),
        data_blocks: vec![
            DataBlock {
                header: "TEST_BLOCK_1".to_string(),
                categories: vec![Category {
                    name: "_test_category".to_string(),
                    row_count: 3,
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            data: EncodedData {
                                encoding: vec![Encoding::ByteArray { data_type: 1 }], // Int8
                                data: ByteBuf::from(vec![1, 2, 3]),
                            },
                            mask: None,
                        },
                        Column {
                            name: "delta_data".to_string(),
                            data: EncodedData {
                                encoding: vec![
                                    Encoding::Delta {
                                        origin: 10,
                                        src_type: 3,
                                    },
                                    Encoding::ByteArray { data_type: 3 },
                                ],
                                data: ByteBuf::from(vec![1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]), // [11, 12, 14]
                            },
                            mask: None,
                        },
                    ],
                }],
            },
            DataBlock {
                header: "TEST_BLOCK_2".to_string(),
                categories: vec![],
            },
        ],
    }
}
