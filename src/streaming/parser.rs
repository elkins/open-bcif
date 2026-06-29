use crate::streaming::{Category, Column, DataBlock};
use anyhow::Context;
use rmp::decode;
use std::io::Read;

pub struct StreamingParser<R: Read> {
    reader: R,
}

pub struct DataBlockHeader {
    pub header: String,
    pub category_count: u32,
}

pub struct CategoryHeader {
    pub name: String,
    pub column_count: u32,
    pub row_count: u32,
}

impl<R: Read> StreamingParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn parse_file_metadata(&mut self) -> anyhow::Result<(String, String, u32)> {
        let _root_map_len =
            decode::read_map_len(&mut self.reader).context("Failed to read root map")?;

        let mut version = String::new();
        let mut encoder = String::new();
        let mut block_count = 0;

        for _ in 0..3 {
            let key = self.read_string()?;
            match key.as_str() {
                "version" => version = self.read_string()?,
                "encoder" => encoder = self.read_string()?,
                "dataBlocks" => block_count = decode::read_array_len(&mut self.reader)?,
                _ => self.skip_value()?,
            }
        }

        Ok((version, encoder, block_count))
    }

    /// Yields just the header of the next DataBlock and prepares for category iteration.
    pub fn next_data_block_header(&mut self) -> anyhow::Result<DataBlockHeader> {
        let _block_map_len = decode::read_map_len(&mut self.reader)?;
        let mut header = String::new();

        for _ in 0..2 {
            let key = self.read_string()?;
            match key.as_str() {
                "header" => header = self.read_string()?,
                "categories" => {
                    let category_count = decode::read_array_len(&mut self.reader)?;
                    return Ok(DataBlockHeader {
                        header,
                        category_count,
                    });
                }
                _ => self.skip_value()?,
            }
        }
        anyhow::bail!("DataBlock incomplete")
    }

    pub fn next_category_header(&mut self) -> anyhow::Result<CategoryHeader> {
        let cat_map_len = decode::read_map_len(&mut self.reader)?;
        let mut name = None;
        let mut row_count = None;

        for i in 0..cat_map_len {
            let key = self.read_string()?;
            match key.as_str() {
                "name" => name = Some(self.read_string()?),
                "rowCount" => row_count = Some(decode::read_int::<u32, _>(&mut self.reader)?),
                "columns" => {
                    let column_count = decode::read_array_len(&mut self.reader)?;

                    // VALIDATION: 'columns' must be the last key to allow streaming its elements.
                    if i != cat_map_len - 1 {
                        anyhow::bail!("Streaming Error: 'columns' is not the last key in category '{:?}'. Full block loading required.", name);
                    }

                    return Ok(CategoryHeader {
                        name: name.context("Category name missing")?,
                        row_count: row_count.context("Category rowCount missing")?,
                        column_count,
                    });
                }
                _ => self.skip_value()?,
            }
        }
        anyhow::bail!("Category incomplete: 'columns' key not found")
    }

    /// Reads the next column fully. Since individual columns are small compared to total categories,
    /// this is the reasonable "leaf" of our streaming.
    pub fn next_column(&mut self) -> anyhow::Result<Column> {
        let column: Column = rmp_serde::from_read(&mut self.reader)?;
        Ok(column)
    }

    /// LEGACY: Maintained for compatibility during refactor.
    /// Uses serde to deserialize the whole Category, which safely handles unordered keys (e.g. from python-mmcif).
    pub fn next_data_block(&mut self) -> anyhow::Result<DataBlock> {
        let header_info = self.next_data_block_header()?;
        let mut categories = Vec::with_capacity(header_info.category_count as usize);
        for _ in 0..header_info.category_count {
            let category: Category = rmp_serde::from_read(&mut self.reader)?;
            categories.push(category);
        }
        Ok(DataBlock {
            header: header_info.header,
            categories,
        })
    }

    fn read_string(&mut self) -> anyhow::Result<String> {
        let len = decode::read_str_len(&mut self.reader)?;
        let mut buf = vec![0u8; len as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }

    fn skip_value(&mut self) -> anyhow::Result<()> {
        let _ = rmpv::decode::read_value(&mut self.reader)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_sample_bcif;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_granular_streaming() {
        let path = "test_granular.bcif";
        create_sample_bcif(path).unwrap();

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let mut parser = StreamingParser::new(reader);

        let (_, _, block_count) = parser.parse_file_metadata().unwrap();
        assert_eq!(block_count, 2);

        let block_header = parser.next_data_block_header().unwrap();
        assert_eq!(block_header.header, "TEST_BLOCK_1");

        let cat_header = parser.next_category_header().unwrap();
        assert_eq!(cat_header.name, "_test_category");
        assert_eq!(cat_header.column_count, 2);

        let col = parser.next_column().unwrap();
        assert_eq!(col.name, "id");

        let col2 = parser.next_column().unwrap();
        assert_eq!(col2.name, "delta_data");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_streaming_parser_missing_metadata() {
        // Create an empty map instead of a valid BCIF
        let path = "test_empty.bcif";
        let file = File::create(path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        rmp::encode::write_map_len(&mut writer, 0).unwrap();
        drop(writer);

        let file = File::open(path).unwrap();
        let mut parser = StreamingParser::new(BufReader::new(file));
        let res = parser.parse_file_metadata();
        assert!(res.is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_streaming_parser_corrupted_data() {
        let path = "test_corrupt.bcif";
        std::fs::write(path, vec![0x93, 0x01, 0x02]).unwrap(); // Invalid MessagePack for our structure

        let file = File::open(path).unwrap();
        let mut parser = StreamingParser::new(BufReader::new(file));
        assert!(parser.parse_file_metadata().is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_streaming_parser_edge_cases() {
        // 1. skip_value in metadata (unknown key "extra")
        let mut buf = Vec::new();
        rmp::encode::write_map_len(&mut buf, 4).unwrap();
        rmp::encode::write_str(&mut buf, "extra").unwrap();
        rmp::encode::write_u32(&mut buf, 123).unwrap(); // will be skipped
        rmp::encode::write_str(&mut buf, "version").unwrap();
        rmp::encode::write_str(&mut buf, "1.0").unwrap();
        rmp::encode::write_str(&mut buf, "encoder").unwrap();
        rmp::encode::write_str(&mut buf, "test").unwrap();
        rmp::encode::write_str(&mut buf, "dataBlocks").unwrap();
        rmp::encode::write_array_len(&mut buf, 0).unwrap();

        let mut parser = StreamingParser::new(std::io::Cursor::new(buf));
        let (v, e, b) = parser.parse_file_metadata().unwrap();
        assert_eq!(v, "1.0");
        assert_eq!(e, "test");
        assert_eq!(b, 0);

        // 2. DataBlock incomplete
        let mut buf = Vec::new();
        rmp::encode::write_map_len(&mut buf, 1).unwrap();
        rmp::encode::write_str(&mut buf, "header").unwrap();
        rmp::encode::write_str(&mut buf, "BLOCK_1").unwrap();
        let mut parser = StreamingParser::new(std::io::Cursor::new(buf));
        assert!(parser.next_data_block_header().is_err());

        // 3. Category incomplete (missing columns)
        let mut buf = Vec::new();
        rmp::encode::write_map_len(&mut buf, 2).unwrap();
        rmp::encode::write_str(&mut buf, "name").unwrap();
        rmp::encode::write_str(&mut buf, "_cat").unwrap();
        rmp::encode::write_str(&mut buf, "rowCount").unwrap();
        rmp::encode::write_u32(&mut buf, 10).unwrap();
        let mut parser = StreamingParser::new(std::io::Cursor::new(buf));
        assert!(parser.next_category_header().is_err());

        // 4. Category columns not last
        let mut buf = Vec::new();
        rmp::encode::write_map_len(&mut buf, 3).unwrap();
        rmp::encode::write_str(&mut buf, "name").unwrap();
        rmp::encode::write_str(&mut buf, "_cat").unwrap();
        rmp::encode::write_str(&mut buf, "columns").unwrap();
        rmp::encode::write_array_len(&mut buf, 0).unwrap();
        rmp::encode::write_str(&mut buf, "rowCount").unwrap();
        rmp::encode::write_u32(&mut buf, 10).unwrap();
        let mut parser = StreamingParser::new(std::io::Cursor::new(buf));
        assert!(parser.next_category_header().is_err());
    }
}
