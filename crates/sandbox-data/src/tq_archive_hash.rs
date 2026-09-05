// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::ContentError;

const MAXIMUM_PATH_BYTES: usize = 256;

pub(crate) fn path_id(normalized_path: &str, context: &'static str) -> Result<u32, ContentError> {
    if !normalized_path.is_ascii() || normalized_path.len() > MAXIMUM_PATH_BYTES {
        return Err(ContentError::InvalidData {
            context,
            detail: "paths must be ASCII and no longer than 256 bytes".to_owned(),
        });
    }

    let path_bytes = normalized_path.as_bytes();
    let mut words = vec![0_u32; path_bytes.len().div_ceil(4) + 2];
    for (index, byte) in path_bytes.iter().copied().enumerate() {
        words[index / 4] |= u32::from(byte) << ((index % 4) * 8);
    }
    let suffix = words.len() - 2;
    words[suffix] = 0x9BE7_4448;
    words[suffix + 1] = 0x66F4_2C48;

    let mut value = 0xF4FA_8928_u32;
    let mut left = 0x7758_B42B_u32;
    let mut right = 0x37A8_470E_u32;

    for word in words {
        value = value.rotate_left(1);
        let mix = 0x267B_0B11 ^ value;
        right ^= word;
        left ^= word;

        let first_factor = (mix.wrapping_add(left) | 0x0204_0801) & 0xBFEF_7FDF;
        let product = u64::from(first_factor) * u64::from(right);
        let mut low = product as u32;
        let high = (product >> 32) as u32;
        if high != 0 {
            low = low.wrapping_add(1);
        }
        let folded = u64::from(low) + u64::from(high);
        low = folded as u32;
        if folded >> 32 != 0 {
            low = low.wrapping_add(1);
        }
        let next_right = low;

        let second_factor = (mix.wrapping_add(right) | 0x0080_4021) & 0x7DFE_FBFF;
        right = next_right;
        let product = u64::from(left) * u64::from(second_factor);
        low = product as u32;
        let high = (product >> 32) as u32;
        let folded = u64::from(high) + u64::from(high);
        let doubled_high = folded as u32;
        if folded >> 32 != 0 {
            low = low.wrapping_add(1);
        }
        let folded = u64::from(low) + u64::from(doubled_high);
        low = folded as u32;
        if folded >> 32 != 0 {
            low = low.wrapping_add(2);
        }
        left = low;
    }

    Ok(right ^ left)
}
