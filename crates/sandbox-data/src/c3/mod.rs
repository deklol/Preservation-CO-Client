// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
mod mesh;
mod motion;

use crate::{ContentError, SliceReader};
pub use mesh::{C3Mesh, C3Vertex, DrawKey, TextureKey, TransparencyKey};
pub use motion::{C3Motion, MotionKey};

const FILE_HEADER_LENGTH: usize = 16;
const CHUNK_HEADER_LENGTH: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Matrix4 {
    pub values: [f32; 16],
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix4 {
    pub const IDENTITY: Self = Self {
        values: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    #[must_use]
    pub fn multiply(self, right: Self) -> Self {
        let mut values = [0.0; 16];
        for row in 0..4 {
            for column in 0..4 {
                values[row * 4 + column] = (0..4)
                    .map(|index| self.values[row * 4 + index] * right.values[index * 4 + column])
                    .sum();
            }
        }
        Self { values }
    }

    #[must_use]
    pub fn transform_point(self, position: Vector3) -> Vector3 {
        Vector3 {
            x: position.x * self.values[0]
                + position.y * self.values[4]
                + position.z * self.values[8]
                + self.values[12],
            y: position.x * self.values[1]
                + position.y * self.values[5]
                + position.z * self.values[9]
                + self.values[13],
            z: position.x * self.values[2]
                + position.y * self.values[6]
                + position.z * self.values[10]
                + self.values[14],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnknownChunk {
    pub tag: [u8; 4],
    pub length: u32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct C3Document {
    pub version: String,
    pub meshes: Vec<C3Mesh>,
    pub motions: Vec<C3Motion>,
    pub unknown_chunks: Vec<UnknownChunk>,
}

impl C3Document {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("C3 document", bytes);
        let raw_header = reader.read_exact(FILE_HEADER_LENGTH)?;
        if !raw_header.starts_with(b"MAXFILE C3 ") {
            return Err(invalid("header does not start with MAXFILE C3"));
        }

        let mut document = Self {
            version: latin1(raw_header).trim_end_matches('\0').trim().to_owned(),
            ..Self::default()
        };

        while reader.remaining() >= CHUNK_HEADER_LENGTH {
            let tag: [u8; 4] = reader
                .read_exact(4)?
                .try_into()
                .expect("chunk tag length was checked");
            let chunk_length = reader.read_u32_le()?;
            let chunk_length_usize = usize::try_from(chunk_length)
                .map_err(|_| invalid(format!("chunk {} is too large", display_tag(tag))))?;
            let chunk = reader.read_exact(chunk_length_usize)?;

            match &tag {
                b"PHY " | b"PHY3" | b"PHY4" | b"PHY5" => {
                    document.meshes.push(C3Mesh::parse(tag, chunk)?)
                }
                b"MOTI" => document.motions.push(C3Motion::parse(chunk)?),
                _ => document.unknown_chunks.push(UnknownChunk {
                    tag,
                    length: chunk_length,
                }),
            }
        }

        if reader.remaining() != 0 {
            return Err(invalid(format!(
                "{} trailing bytes cannot form a chunk header",
                reader.remaining()
            )));
        }

        Ok(document)
    }
}

fn read_vector2(reader: &mut SliceReader<'_>) -> Result<Vector2, ContentError> {
    Ok(Vector2 {
        x: reader.read_f32_le()?,
        y: reader.read_f32_le()?,
    })
}

fn read_vector3(reader: &mut SliceReader<'_>) -> Result<Vector3, ContentError> {
    Ok(Vector3 {
        x: reader.read_f32_le()?,
        y: reader.read_f32_le()?,
        z: reader.read_f32_le()?,
    })
}

fn read_matrix(reader: &mut SliceReader<'_>) -> Result<Matrix4, ContentError> {
    let mut values = [0.0; 16];
    for value in &mut values {
        *value = reader.read_f32_le()?;
    }
    Ok(Matrix4 { values })
}

fn read_sized_string(reader: &mut SliceReader<'_>) -> Result<String, ContentError> {
    let length = usize::try_from(reader.read_u32_le()?)
        .map_err(|_| invalid("string length does not fit this platform"))?;
    if length > 4_096 {
        return Err(invalid(format!(
            "string length {length} exceeds 4096 bytes"
        )));
    }
    Ok(latin1(reader.read_exact(length)?))
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn display_tag(tag: [u8; 4]) -> String {
    latin1(&tag)
}

fn checked_count(value: u32, element_size: usize, remaining: usize) -> Result<usize, ContentError> {
    let count = usize::try_from(value).map_err(|_| invalid("count does not fit this platform"))?;
    let required = count
        .checked_mul(element_size)
        .ok_or_else(|| invalid("element byte count overflowed"))?;
    if required > remaining {
        return Err(invalid(format!(
            "{count} elements require at least {required} bytes; only {remaining} remain"
        )));
    }
    Ok(count)
}

fn invalid(detail: impl Into<String>) -> ContentError {
    ContentError::InvalidData {
        context: "C3",
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_c3_payloads() {
        let error = C3Document::parse(&[0; 16]).unwrap_err();
        assert!(error.to_string().contains("MAXFILE C3"));
    }

    #[test]
    fn accepts_an_empty_c3_container() {
        let mut bytes = [0_u8; 16];
        bytes[..11].copy_from_slice(b"MAXFILE C3 ");
        let document = C3Document::parse(&bytes).unwrap();
        assert!(document.meshes.is_empty());
        assert!(document.motions.is_empty());
    }
}
