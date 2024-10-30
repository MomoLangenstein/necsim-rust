use tiff::{decoder::DecodingResult, tags::SampleFormat};

#[expect(clippy::module_name_repetitions)]
pub trait TiffDataType:
    Clone
    + std::fmt::Debug
    + Default
    + std::str::FromStr<Err: std::error::Error + Send + Sync + 'static>
    + std::cmp::PartialEq
{
    const BIT_WIDTH: u8;
    const SAMPLE_FORMAT: SampleFormat;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>>;
}

impl TiffDataType for u8 {
    const BIT_WIDTH: u8 = 8_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Uint;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::U8(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for u16 {
    const BIT_WIDTH: u8 = 16_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Uint;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::U16(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for u32 {
    const BIT_WIDTH: u8 = 32_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Uint;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::U32(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for u64 {
    const BIT_WIDTH: u8 = 64_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Uint;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::U64(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for f32 {
    const BIT_WIDTH: u8 = 32_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::IEEEFP;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::F32(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for f64 {
    const BIT_WIDTH: u8 = 64_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::IEEEFP;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::F64(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for i8 {
    const BIT_WIDTH: u8 = 8_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Int;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::I8(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for i16 {
    const BIT_WIDTH: u8 = 16_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Int;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::I16(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for i32 {
    const BIT_WIDTH: u8 = 32_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Int;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::I32(data) => Some(data),
            _ => None,
        }
    }
}

impl TiffDataType for i64 {
    const BIT_WIDTH: u8 = 64_u8;
    const SAMPLE_FORMAT: SampleFormat = SampleFormat::Int;

    fn decoding_result_to_data(result: DecodingResult) -> Option<Vec<Self>> {
        match result {
            DecodingResult::I64(data) => Some(data),
            _ => None,
        }
    }
}
