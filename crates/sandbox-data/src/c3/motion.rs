// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{Matrix4, checked_count, invalid, read_matrix};
use crate::{ContentError, SliceReader};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MotionKey {
    pub frame: u32,
    pub bone_matrices: Vec<Matrix4>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct C3Motion {
    pub bone_count: u32,
    pub frame_count: u32,
    pub keys: Vec<MotionKey>,
    pub morph_count: u32,
    pub morph_values: Vec<f32>,
}

impl C3Motion {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("C3 motion", bytes);
        let bone_count = reader.read_u32_le()?;
        let frame_count = reader.read_u32_le()?;
        let marker = reader.read_exact(4)?;
        let keys = match marker {
            b"KKEY" => read_matrix_keys(&mut reader, bone_count)?,
            b"ZKEY" => read_quaternion_keys(&mut reader, bone_count)?,
            b"XKEY" => read_compact_matrix_keys(&mut reader, bone_count)?,
            _ => {
                reader.rewind(4)?;
                read_legacy_keys(&mut reader, bone_count, frame_count)?
            }
        };

        let morph_count = if reader.remaining() >= 4 {
            reader.read_u32_le()?
        } else {
            0
        };
        let morph_value_count = morph_count
            .checked_mul(frame_count)
            .ok_or_else(|| invalid("motion morph count overflowed"))?;
        let morph_value_count = checked_count(morph_value_count, 4, reader.remaining())?;
        let morph_values = (0..morph_value_count)
            .map(|_| reader.read_f32_le())
            .collect::<Result<Vec<_>, _>>()?;
        if reader.remaining() != 0 {
            return Err(invalid(format!(
                "motion has {} unparsed bytes",
                reader.remaining()
            )));
        }

        Ok(Self {
            bone_count,
            frame_count,
            keys,
            morph_count,
            morph_values,
        })
    }

    #[must_use]
    pub fn matrix_at(&self, bone: u32, frame: f32) -> Matrix4 {
        if bone >= self.bone_count || self.keys.is_empty() {
            return Matrix4::IDENTITY;
        }
        let wrapped = if self.frame_count > 0 {
            frame.rem_euclid(self.frame_count as f32)
        } else {
            frame
        };
        let mut before = &self.keys[0];
        let mut after = self.keys.last().expect("motion keys are non-empty");
        for key in &self.keys {
            if key.frame as f32 <= wrapped {
                before = key;
            }
            if key.frame as f32 >= wrapped {
                after = key;
                break;
            }
        }
        let Some(left) = before.bone_matrices.get(bone as usize).copied() else {
            return Matrix4::IDENTITY;
        };
        let Some(right) = after.bone_matrices.get(bone as usize).copied() else {
            return left;
        };
        if before.frame == after.frame {
            return left;
        }
        let amount = (wrapped - before.frame as f32) / (after.frame - before.frame) as f32;
        Matrix4 {
            values: std::array::from_fn(|index| {
                left.values[index] + (right.values[index] - left.values[index]) * amount
            }),
        }
    }
}

fn read_matrix_keys(
    reader: &mut SliceReader<'_>,
    bone_count: u32,
) -> Result<Vec<MotionKey>, ContentError> {
    let count = reader.read_u32_le()?;
    let bytes_per_key = 4_usize
        .checked_add(matrix_bytes(bone_count)?)
        .ok_or_else(|| invalid("matrix key size overflowed"))?;
    let count = checked_count(count, bytes_per_key, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = reader.read_u32_le()?;
        keys.push(MotionKey {
            frame,
            bone_matrices: read_matrices(reader, bone_count)?,
        });
    }
    Ok(keys)
}

fn read_quaternion_keys(
    reader: &mut SliceReader<'_>,
    bone_count: u32,
) -> Result<Vec<MotionKey>, ContentError> {
    let count = reader.read_u32_le()?;
    let bone_count_usize = usize::try_from(bone_count)
        .map_err(|_| invalid("bone count does not fit this platform"))?;
    let bytes_per_key = 2_usize
        .checked_add(
            bone_count_usize
                .checked_mul(28)
                .ok_or_else(|| invalid("quaternion key size overflowed"))?,
        )
        .ok_or_else(|| invalid("quaternion key size overflowed"))?;
    let count = checked_count(count, bytes_per_key, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = u32::from(reader.read_u16_le()?);
        let mut matrices = Vec::with_capacity(bone_count_usize);
        for _ in 0..bone_count_usize {
            let quaternion = [
                reader.read_f32_le()?,
                reader.read_f32_le()?,
                reader.read_f32_le()?,
                reader.read_f32_le()?,
            ];
            let translation = [
                reader.read_f32_le()?,
                reader.read_f32_le()?,
                reader.read_f32_le()?,
            ];
            matrices.push(quaternion_matrix(quaternion, translation));
        }
        keys.push(MotionKey {
            frame,
            bone_matrices: matrices,
        });
    }
    Ok(keys)
}

fn read_compact_matrix_keys(
    reader: &mut SliceReader<'_>,
    bone_count: u32,
) -> Result<Vec<MotionKey>, ContentError> {
    let count = reader.read_u32_le()?;
    let bone_count_usize = usize::try_from(bone_count)
        .map_err(|_| invalid("bone count does not fit this platform"))?;
    let bytes_per_key = 2_usize
        .checked_add(
            bone_count_usize
                .checked_mul(48)
                .ok_or_else(|| invalid("compact matrix key size overflowed"))?,
        )
        .ok_or_else(|| invalid("compact matrix key size overflowed"))?;
    let count = checked_count(count, bytes_per_key, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = u32::from(reader.read_u16_le()?);
        let mut matrices = Vec::with_capacity(bone_count_usize);
        for _ in 0..bone_count_usize {
            let mut values = Matrix4::IDENTITY.values;
            for row in 0..3 {
                for column in 0..3 {
                    values[row * 4 + column] = reader.read_f32_le()?;
                }
            }
            values[12] = reader.read_f32_le()?;
            values[13] = reader.read_f32_le()?;
            values[14] = reader.read_f32_le()?;
            matrices.push(Matrix4 { values });
        }
        keys.push(MotionKey {
            frame,
            bone_matrices: matrices,
        });
    }
    Ok(keys)
}

fn read_legacy_keys(
    reader: &mut SliceReader<'_>,
    bone_count: u32,
    frame_count: u32,
) -> Result<Vec<MotionKey>, ContentError> {
    let bone_count_usize = usize::try_from(bone_count)
        .map_err(|_| invalid("bone count does not fit this platform"))?;
    let frame_count_usize = usize::try_from(frame_count)
        .map_err(|_| invalid("frame count does not fit this platform"))?;
    let matrix_count = bone_count
        .checked_mul(frame_count)
        .ok_or_else(|| invalid("legacy motion matrix count overflowed"))?;
    checked_count(matrix_count, 64, reader.remaining())?;
    let mut keys = (0..frame_count_usize)
        .map(|frame| MotionKey {
            frame: u32::try_from(frame).expect("frame count originated as u32"),
            bone_matrices: vec![Matrix4::IDENTITY; bone_count_usize],
        })
        .collect::<Vec<_>>();
    for bone in 0..bone_count_usize {
        for key in &mut keys {
            key.bone_matrices[bone] = read_matrix(reader)?;
        }
    }
    Ok(keys)
}

fn read_matrices(
    reader: &mut SliceReader<'_>,
    bone_count: u32,
) -> Result<Vec<Matrix4>, ContentError> {
    let count = usize::try_from(bone_count)
        .map_err(|_| invalid("bone count does not fit this platform"))?;
    (0..count).map(|_| read_matrix(reader)).collect()
}

fn matrix_bytes(bone_count: u32) -> Result<usize, ContentError> {
    usize::try_from(bone_count)
        .ok()
        .and_then(|count| count.checked_mul(64))
        .ok_or_else(|| invalid("matrix byte count overflowed"))
}

fn quaternion_matrix(quaternion: [f32; 4], translation: [f32; 3]) -> Matrix4 {
    let [x, y, z, w] = quaternion;
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let zw = z * w;
    let zx = z * x;
    let yw = y * w;
    let yz = y * z;
    let xw = x * w;
    Matrix4 {
        values: [
            1.0 - 2.0 * (yy + zz),
            2.0 * (xy + zw),
            2.0 * (zx - yw),
            0.0,
            2.0 * (xy - zw),
            1.0 - 2.0 * (zz + xx),
            2.0 * (yz + xw),
            0.0,
            2.0 * (zx + yw),
            2.0 * (yz - xw),
            1.0 - 2.0 * (yy + xx),
            0.0,
            translation[0],
            translation[1],
            translation[2],
            1.0,
        ],
    }
}
