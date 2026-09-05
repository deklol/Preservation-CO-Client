// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::ContentError;

#[derive(Debug)]
pub struct SliceReader<'a> {
    context: &'static str,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SliceReader<'a> {
    #[must_use]
    pub const fn new(context: &'static str, bytes: &'a [u8]) -> Self {
        Self {
            context,
            bytes,
            offset: 0,
        }
    }

    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    pub fn read_exact(&mut self, length: usize) -> Result<&'a [u8], ContentError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(ContentError::Truncated {
                context: self.context,
                offset: self.offset,
                needed: length,
                available: self.bytes.len().saturating_sub(self.offset),
            })?;

        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    pub fn peek_exact(&self, length: usize) -> Result<&'a [u8], ContentError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(ContentError::Truncated {
                context: self.context,
                offset: self.offset,
                needed: length,
                available: self.bytes.len().saturating_sub(self.offset),
            })?;
        Ok(&self.bytes[self.offset..end])
    }

    pub fn rewind(&mut self, length: usize) -> Result<(), ContentError> {
        self.offset = self
            .offset
            .checked_sub(length)
            .ok_or(ContentError::InvalidData {
                context: self.context,
                detail: format!("cannot rewind {length} bytes from offset {}", self.offset),
            })?;
        Ok(())
    }

    pub fn read_u16_le(&mut self) -> Result<u16, ContentError> {
        let bytes: [u8; 2] = self.read_exact(2)?.try_into().expect("length was checked");
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_i16_le(&mut self) -> Result<i16, ContentError> {
        let bytes: [u8; 2] = self.read_exact(2)?.try_into().expect("length was checked");
        Ok(i16::from_le_bytes(bytes))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, ContentError> {
        let bytes: [u8; 4] = self.read_exact(4)?.try_into().expect("length was checked");
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_i32_le(&mut self) -> Result<i32, ContentError> {
        let bytes: [u8; 4] = self.read_exact(4)?.try_into().expect("length was checked");
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn read_f32_le(&mut self) -> Result<f32, ContentError> {
        let bytes: [u8; 4] = self.read_exact(4)?.try_into().expect("length was checked");
        Ok(f32::from_le_bytes(bytes))
    }

    pub fn skip(&mut self, length: usize) -> Result<(), ContentError> {
        self.read_exact(length).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reader_reports_the_exact_failed_offset() {
        let mut reader = SliceReader::new("fixture", &[0x34, 0x12, 0xAA]);
        assert_eq!(reader.read_u16_le().unwrap(), 0x1234);

        let error = reader.read_u32_le().unwrap_err();
        assert_eq!(
            error,
            ContentError::Truncated {
                context: "fixture",
                offset: 2,
                needed: 4,
                available: 1,
            }
        );
    }
}
