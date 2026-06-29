use criterion::{criterion_group, criterion_main, Criterion};
use open_bcif::encoding::{EncodedData, Encoding};
use open_bcif::streaming::parser::StreamingParser;
use open_bcif::streaming::{Category, Column, DataBlock, File};
use rmp_serde::encode::write_named;
use serde_bytes::ByteBuf;
use std::io::Cursor;

fn create_in_memory_bcif(num_blocks: usize) -> Vec<u8> {
    let mut data_blocks = Vec::new();
    for i in 0..num_blocks {
        data_blocks.push(DataBlock {
            header: format!("BLOCK_{}", i),
            categories: vec![Category {
                name: "_test".to_string(),
                row_count: 1000,
                columns: vec![Column {
                    name: "val".to_string(),
                    data: EncodedData {
                        encoding: vec![Encoding::ByteArray { data_type: 3 }],
                        data: ByteBuf::from(vec![0u8; 4000]),
                    },
                    mask: None,
                }],
            }],
        });
    }

    let bcif = File {
        version: env!("CARGO_PKG_VERSION").to_string(),
        encoder: "bench".to_string(),
        data_blocks,
    };

    let mut buf = Vec::new();
    write_named(&mut buf, &bcif).unwrap();
    buf
}

fn bench_streaming_parser(c: &mut Criterion) {
    let bcif_data = create_in_memory_bcif(100);

    c.bench_function("stream_100_blocks", |b| {
        b.iter(|| {
            let cursor = Cursor::new(&bcif_data);
            let mut parser = StreamingParser::new(cursor);
            let (_, _, count) = parser.parse_file_metadata().unwrap();
            for _ in 0..count {
                let _ = parser.next_data_block().unwrap();
            }
        })
    });
}

criterion_group!(benches, bench_streaming_parser);
criterion_main!(benches);
