use crate::streaming::parser::StreamingParser;
use crate::streaming::File as BcifFile;
use anyhow::Context;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub fn merge(inputs: &[String], output_path: &str) -> anyhow::Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("No input files provided for merge");
    }

    println!("Merging {} files into {}", inputs.len(), output_path);

    let mut all_blocks = Vec::new();
    let mut final_version = String::new();
    let mut final_encoder = String::new();

    for (i, input_path) in inputs.iter().enumerate() {
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open input file: {}", input_path))?;
        let reader = BufReader::new(file);
        let mut parser = StreamingParser::new(reader);

        let (version, encoder, block_count) = parser.parse_file_metadata()?;

        if i == 0 {
            final_version = version;
            final_encoder = format!("{} (merged)", encoder);
        }

        println!("  Reading {} blocks from {}", block_count, input_path);
        for _ in 0..block_count {
            all_blocks.push(parser.next_data_block()?);
        }
    }

    let out_file = File::create(output_path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(out_file);

    let merged_file = BcifFile {
        version: final_version,
        encoder: final_encoder,
        data_blocks: all_blocks,
    };

    rmp_serde::encode::write_named(&mut writer, &merged_file)?;
    println!("Merge successful.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_sample_bcif;
    use std::fs;

    #[test]
    fn test_merge_functional() {
        let in1 = "test_merge_1.bcif";
        let in2 = "test_merge_2.bcif";
        let out = "test_merged.bcif";

        create_sample_bcif(in1).unwrap();
        create_sample_bcif(in2).unwrap();

        merge(&[in1.to_string(), in2.to_string()], out).unwrap();

        // Verify output
        let file = File::open(out).unwrap();
        let reader = BufReader::new(file);
        let mut parser = StreamingParser::new(reader);
        let (_, _, block_count) = parser.parse_file_metadata().unwrap();
        assert_eq!(block_count, 4); // 2 from each sample

        // Clean up
        fs::remove_file(in1).unwrap();
        fs::remove_file(in2).unwrap();
        fs::remove_file(out).unwrap();
    }
    #[test]
    fn test_merge_empty_inputs() {
        let out = "test_merged_empty.bcif";
        let result = merge(&[], out);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No input files provided for merge"
        );
    }
}
