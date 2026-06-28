//! # open-bcif
//!
//! `open-bcif` is a high-performance streaming toolkit for BinaryCIF (BCIF) files.
//! It is designed for parsing, validating, and modifying large BCIF files efficiently 
//! in environments where memory overhead must be kept low.
//!
//! ## Example
//! 
//! Read the metadata and data blocks from a BinaryCIF file stream using the `StreamingParser`:
//!
//! ```no_run
//! use std::fs::File;
//! use std::io::BufReader;
//! use open_bcif::streaming::parser::StreamingParser;
//! 
//! fn main() -> anyhow::Result<()> {
//!     let file = File::open("example.bcif")?;
//!     let reader = BufReader::new(file);
//!     let mut parser = StreamingParser::new(reader);
//! 
//!     let (version, encoder, block_count) = parser.parse_file_metadata()?;
//!     println!("BCIF Version: {}, Encoder: {}", version, encoder);
//! 
//!     for _ in 0..block_count {
//!         let data_block = parser.next_data_block_header()?;
//!         println!("Block Header: {}", data_block.header);
//!         
//!         for _ in 0..data_block.category_count {
//!             let category = parser.next_category_header()?;
//!             println!("Category: {} with {} rows", category.name, category.row_count);
//!             
//!             // Process columns...
//!             for _ in 0..category.column_count {
//!                 let column = parser.next_column()?;
//!                 println!("  Column: {}", column.name);
//!             }
//!         }
//!     }
//! 
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod commands;
pub mod encoding;
pub mod streaming;
pub mod test_utils;
