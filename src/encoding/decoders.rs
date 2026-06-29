pub fn decode_byte_array(data: &[u8], data_type: i32) -> anyhow::Result<Vec<f64>> {
    match data_type {
        1 => Ok(data.iter().map(|&x| x as i8 as f64).collect()), // Int8
        2 => {
            let mut result = Vec::with_capacity(data.len() / 2);
            for chunk in data.chunks_exact(2) {
                result.push(i16::from_le_bytes([chunk[0], chunk[1]]) as f64);
            }
            Ok(result)
        } // Int16
        3 => {
            let mut result = Vec::with_capacity(data.len() / 4);
            for chunk in data.chunks_exact(4) {
                result.push(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
            Ok(result)
        } // Int32
        4 => Ok(data.iter().map(|&x| x as f64).collect()),       // Uint8
        5 => {
            let mut result = Vec::with_capacity(data.len() / 2);
            for chunk in data.chunks_exact(2) {
                result.push(u16::from_le_bytes([chunk[0], chunk[1]]) as f64);
            }
            Ok(result)
        } // Uint16
        6 => {
            let mut result = Vec::with_capacity(data.len() / 4);
            for chunk in data.chunks_exact(4) {
                result.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
            Ok(result)
        } // Uint32
        32 => {
            let mut result = Vec::with_capacity(data.len() / 4);
            for chunk in data.chunks_exact(4) {
                result.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
            Ok(result)
        } // Float32
        33 => {
            let mut result = Vec::with_capacity(data.len() / 8);
            for chunk in data.chunks_exact(8) {
                result.push(f64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]));
            }
            Ok(result)
        } // Float64
        _ => anyhow::bail!("Unsupported data type: {}", data_type),
    }
}

#[allow(dead_code)]
pub fn decode_delta(data: Vec<f64>, origin: i32) -> Vec<f64> {
    let mut current = origin as f64;
    let mut result = Vec::with_capacity(data.len());
    for val in data {
        current += val;
        result.push(current);
    }
    result
}

#[allow(dead_code)]
pub fn decode_run_length(data: Vec<f64>, src_size: i32) -> Vec<f64> {
    let mut result = Vec::with_capacity(src_size as usize);
    for chunk in data.chunks_exact(2) {
        let value = chunk[0];
        let count = chunk[1] as usize;
        for _ in 0..count {
            result.push(value);
        }
    }
    result
}

#[allow(dead_code)]
pub fn decode_fixed_point(data: Vec<f64>, factor: f64) -> Vec<f64> {
    data.into_iter().map(|x| x / factor).collect()
}

#[allow(dead_code)]
pub fn decode_interval_quantization(
    data: Vec<f64>,
    min: f64,
    max: f64,
    num_steps: i32,
) -> Vec<f64> {
    let delta = (max - min) / (num_steps as f64);
    data.into_iter()
        .map(|x| min + (x + 0.5) * delta)
        .collect()
}

#[allow(dead_code)]
pub fn decode_integer_packing(
    data: &[u8],
    byte_count: i32,
    is_unsigned: bool,
    src_size: i32,
) -> anyhow::Result<Vec<f64>> {
    let mut result = Vec::with_capacity(src_size as usize);
    let mut i = 0;

    if is_unsigned {
        let limit = match byte_count {
            1 => 0xFFu32,
            2 => 0xFFFFu32,
            _ => anyhow::bail!("Unsupported byte count for IntegerPacking: {}", byte_count),
        };

        while i < data.len() {
            let mut value: u32 = 0;
            loop {
                let val = match byte_count {
                    1 => data[i] as u32,
                    2 => {
                        if i + 1 >= data.len() {
                            anyhow::bail!("Unexpected end of data in IntegerPacking");
                        }
                        u16::from_le_bytes([data[i], data[i + 1]]) as u32
                    }
                    _ => unreachable!(),
                };
                i += byte_count as usize;
                value += val;
                if val != limit || i >= data.len() {
                    break;
                }
            }
            result.push(value as f64);
        }
    } else {
        let pos_limit = match byte_count {
            1 => 0x7Fi32,
            2 => 0x7FFFi32,
            _ => anyhow::bail!("Unsupported byte count for IntegerPacking: {}", byte_count),
        };
        let neg_limit = -pos_limit;

        while i < data.len() {
            let mut value: i32 = 0;
            loop {
                let val = match byte_count {
                    1 => data[i] as i8 as i32,
                    2 => {
                        if i + 1 >= data.len() {
                            anyhow::bail!("Unexpected end of data in IntegerPacking");
                        }
                        i16::from_le_bytes([data[i], data[i + 1]]) as i32
                    }
                    _ => unreachable!(),
                };
                i += byte_count as usize;
                value += val;
                if (val != pos_limit && val != neg_limit) || i >= data.len() {
                    break;
                }
            }
            result.push(value as f64);
        }
    }

    Ok(result)
}

#[allow(dead_code)]
pub fn decode_string_array(indices: &[i32], offsets: &[i32], string_data: &str) -> Vec<String> {
    let mut result = Vec::with_capacity(indices.len());
    for &idx in indices {
        if idx == -1 {
            result.push(".".to_string()); // Standard CIF null/unknown placeholder
            continue;
        }
        if idx == -2 {
            result.push("?".to_string()); // Standard CIF omitted placeholder
            continue;
        }

        let start = offsets[idx as usize] as usize;
        let end = offsets[(idx + 1) as usize] as usize;
        result.push(string_data[start..end].to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(vec![0, 1, 255], 1, vec![0.0, 1.0, -1.0])] // Int8
    #[case(vec![1, 0, 255, 255], 2, vec![1.0, -1.0])] // Int16
    #[case(vec![1, 0, 0, 0, 255, 255, 255, 255], 6, vec![1.0, 4294967295.0])] // Uint32
    fn test_decode_byte_array_matrix(#[case] data: Vec<u8>, #[case] data_type: i32, #[case] expected: Vec<f64>) {
        let decoded = decode_byte_array(&data, data_type).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_decode_string_array() {
        let string_data = "ABCDEF";
        let offsets = vec![0, 3, 6]; // "ABC", "DEF"
        let indices = vec![0, 1, 0, -1];
        let decoded = decode_string_array(&indices, &offsets, string_data);
        assert_eq!(decoded, vec!["ABC", "DEF", "ABC", "."]);
    }

    #[test]
    fn test_decode_integer_packing_signed_8bit() {
        // [127, 127, 10, -127, -127, -5, 42] -> [264, -259, 42]
        let data = vec![127, 127, 10, -127i8 as u8, -127i8 as u8, -5i8 as u8, 42];
        let decoded = decode_integer_packing(&data, 1, false, 3).unwrap();
        assert_eq!(decoded, vec![264.0, -259.0, 42.0]);
    }

    #[test]
    fn test_decode_integer_packing_unsigned_16bit() {
        // [65535, 65535, 10, 42] -> [131080, 42]
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 10, 0, 42, 0];
        let decoded = decode_integer_packing(&data, 2, true, 2).unwrap();
        assert_eq!(decoded, vec![131080.0, 42.0]);
    }

    #[test]
    fn test_decode_integer_packing_errors() {
        let data = vec![1, 2, 3];
        // Unexpected end of data for 16-bit packing
        let res = decode_integer_packing(&data, 2, true, 2);
        assert!(res.is_err());

        // Unsupported byte count
        let res = decode_integer_packing(&data, 4, true, 2);
        assert!(res.is_err());
    }

    #[test]
    fn test_decode_byte_array_unsupported() {
        let data = vec![0, 0, 0, 0];
        let res = decode_byte_array(&data, 999);
        assert!(res.is_err());
    }

    #[test]
    fn test_decode_delta() {
        let data = vec![1.0, 1.0, 2.0];
        let decoded = decode_delta(data, 10);
        assert_eq!(decoded, vec![11.0, 12.0, 14.0]);
    }

    #[test]
    fn test_decode_run_length() {
        let data = vec![1.0, 3.0, 2.0, 1.0]; // Three 1s, one 2
        let decoded = decode_run_length(data, 4);
        assert_eq!(decoded, vec![1.0, 1.0, 1.0, 2.0]);
    }

    #[test]
    fn test_decode_fixed_point() {
        let data = vec![123.0, 456.0];
        let decoded = decode_fixed_point(data, 100.0);
        assert_eq!(decoded, vec![1.23, 4.56]);
    }

    #[test]
    fn test_decode_interval_quantization() {
        let data = vec![0.0, 99.0];
        let decoded = decode_interval_quantization(data, 0.0, 100.0, 100);
        // delta = 1.0. 
        // 0.0 -> 0.0 + (0.0 + 0.5) * 1.0 = 0.5
        // 99.0 -> 0.0 + (99.0 + 0.5) * 1.0 = 99.5
        assert_eq!(decoded, vec![0.5, 99.5]);
    }

    #[test]
    fn test_decode_byte_array_float32() {
        let val1 = 12.34f32.to_le_bytes();
        let val2 = 56.78f32.to_le_bytes();
        let data = vec![
            val1[0], val1[1], val1[2], val1[3],
            val2[0], val2[1], val2[2], val2[3],
        ];
        let decoded = decode_byte_array(&data, 32).unwrap();
        assert!((decoded[0] - 12.34f32 as f64).abs() < 1e-6);
        assert!((decoded[1] - 56.78f32 as f64).abs() < 1e-6);
    }

    #[test]
    fn test_decode_integer_packing_errors_unsupported_byte_count_signed() {
        let data = vec![1, 2, 3];
        // Unsupported byte count for signed
        let res = decode_integer_packing(&data, 4, false, 2);
        assert!(res.is_err());
    }

    #[test]
    fn test_decode_integer_packing_unexpected_end_signed() {
        let data = vec![1];
        // Unexpected end for 16-bit signed
        let res = decode_integer_packing(&data, 2, false, 2);
        assert!(res.is_err());
    }

    #[test]
    fn test_decode_integer_packing_signed_16bit() {
        // [32767, 32767, 10, 42] -> [65544, 42]
        let data = vec![0xFF, 0x7F, 0xFF, 0x7F, 10, 0, 42, 0];
        let decoded = decode_integer_packing(&data, 2, false, 2).unwrap();
        assert_eq!(decoded, vec![65544.0, 42.0]);
    }

    #[test]
    fn test_decode_string_array_unknown_placeholders() {
        let string_data = "ABCDEF";
        let offsets = vec![0, 3, 6]; 
        let indices = vec![0, -1, -2, 1];
        let decoded = decode_string_array(&indices, &offsets, string_data);
        assert_eq!(decoded, vec!["ABC", ".", "?", "DEF"]);
    }
}
