// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{
    Matrix4, Vector2, Vector3, checked_count, invalid, read_matrix, read_sized_string,
    read_vector2, read_vector3,
};
use crate::{ContentError, SliceReader};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct C3Vertex {
    pub position: Vector3,
    pub texture_coordinate: Vector2,
    pub colour: u32,
    pub bone_indices: [u32; 2],
    pub bone_weights: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransparencyKey {
    pub frame: i32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DrawKey {
    pub frame: i32,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextureKey {
    pub frame: i32,
    pub texture: i32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct C3Mesh {
    pub name: String,
    pub blend_count: u32,
    pub opaque_vertex_count: u32,
    pub alpha_vertex_count: u32,
    pub vertices: Vec<C3Vertex>,
    pub opaque_indices: Vec<u16>,
    pub alpha_indices: Vec<u16>,
    pub texture_name: String,
    pub bounds_minimum: Vector3,
    pub bounds_maximum: Vector3,
    pub initial_matrix: Matrix4,
    pub texture_rows: u32,
    pub transparency_keys: Vec<TransparencyKey>,
    pub draw_keys: Vec<DrawKey>,
    pub texture_keys: Vec<TextureKey>,
    pub extension_tag: Option<[u8; 4]>,
}

impl C3Mesh {
    pub(super) fn parse(tag: [u8; 4], bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("C3 mesh", bytes);
        let name = read_sized_string(&mut reader)?;
        let blend_count = reader.read_u32_le()?;
        let opaque_vertex_count = reader.read_u32_le()?;
        let alpha_vertex_count = reader.read_u32_le()?;
        let vertex_count = opaque_vertex_count
            .checked_add(alpha_vertex_count)
            .ok_or_else(|| invalid("vertex count overflowed"))?;
        let morph_positions = if &tag == b"PHY " { 4 } else { 1 };
        let extra_vertex_bytes = match &tag {
            b"PHY3" => 12,
            b"PHY5" => 20,
            _ => 0,
        };
        let minimum_vertex_bytes = morph_positions * 12 + 28 + extra_vertex_bytes;
        let vertex_count = checked_count(vertex_count, minimum_vertex_bytes, reader.remaining())?;
        let mut vertices = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let position = read_vector3(&mut reader)?;
            for _ in 1..morph_positions {
                reader.skip(12)?;
            }
            let texture_coordinate = read_vector2(&mut reader)?;
            let colour = reader.read_u32_le()?;
            let bone_indices = [reader.read_u32_le()?, reader.read_u32_le()?];
            let bone_weights = [reader.read_f32_le()?, reader.read_f32_le()?];
            reader.skip(extra_vertex_bytes)?;
            vertices.push(C3Vertex {
                position,
                texture_coordinate,
                colour,
                bone_indices,
                bone_weights,
            });
        }

        let opaque_triangle_count = reader.read_u32_le()?;
        let alpha_triangle_count = reader.read_u32_le()?;
        let opaque_indices = read_indices(&mut reader, opaque_triangle_count)?;
        let alpha_indices = read_indices(&mut reader, alpha_triangle_count)?;
        let texture_name = read_sized_string(&mut reader)?;
        let bounds_minimum = read_vector3(&mut reader)?;
        let bounds_maximum = read_vector3(&mut reader)?;
        let initial_matrix = read_matrix(&mut reader)?;

        let texture_rows = if reader.remaining() >= 4 {
            reader.read_u32_le()?
        } else {
            1
        };
        let transparency_keys = if reader.remaining() >= 4 {
            read_transparency_keys(&mut reader)?
        } else {
            Vec::new()
        };
        let draw_keys = if reader.remaining() >= 4 {
            read_draw_keys(&mut reader)?
        } else {
            Vec::new()
        };
        let texture_keys = if reader.remaining() >= 4 {
            read_texture_keys(&mut reader)?
        } else {
            Vec::new()
        };
        if reader.remaining() >= 4 && reader.peek_exact(4)? == b"STEP" {
            reader.skip(12)?;
        }
        let extension_tag = if reader.remaining() == 4 && reader.peek_exact(4)? == b"2SID" {
            Some(
                reader
                    .read_exact(4)?
                    .try_into()
                    .expect("extension tag length was checked"),
            )
        } else {
            None
        };
        if reader.remaining() != 0 {
            return Err(invalid(format!(
                "mesh {name} has {} unparsed bytes",
                reader.remaining()
            )));
        }

        Ok(Self {
            name,
            blend_count,
            opaque_vertex_count,
            alpha_vertex_count,
            vertices,
            opaque_indices,
            alpha_indices,
            texture_name,
            bounds_minimum,
            bounds_maximum,
            initial_matrix,
            texture_rows,
            transparency_keys,
            draw_keys,
            texture_keys,
            extension_tag,
        })
    }

    #[must_use]
    pub fn transformed_vertex_position(
        &self,
        vertex: &C3Vertex,
        motion: &super::C3Motion,
        attachment: Matrix4,
        frame: f32,
    ) -> Vector3 {
        for influence in 0..2 {
            if vertex.bone_weights[influence] <= 0.0 {
                continue;
            }
            let transform = self
                .initial_matrix
                .multiply(motion.matrix_at(vertex.bone_indices[influence], frame))
                .multiply(attachment);
            return transform.transform_point(vertex.position);
        }
        self.initial_matrix
            .multiply(attachment)
            .transform_point(vertex.position)
    }

    #[must_use]
    pub fn transformed_bounds(
        &self,
        motion: &super::C3Motion,
        attachment: Matrix4,
        frame: f32,
    ) -> Option<(Vector3, Vector3)> {
        let first = self.vertices.first()?;
        let first = self.transformed_vertex_position(first, motion, attachment, frame);
        let mut minimum = first;
        let mut maximum = first;
        for vertex in &self.vertices[1..] {
            let point = self.transformed_vertex_position(vertex, motion, attachment, frame);
            minimum.x = minimum.x.min(point.x);
            minimum.y = minimum.y.min(point.y);
            minimum.z = minimum.z.min(point.z);
            maximum.x = maximum.x.max(point.x);
            maximum.y = maximum.y.max(point.y);
            maximum.z = maximum.z.max(point.z);
        }
        Some((minimum, maximum))
    }
}

fn read_indices(
    reader: &mut SliceReader<'_>,
    triangle_count: u32,
) -> Result<Vec<u16>, ContentError> {
    let count = triangle_count
        .checked_mul(3)
        .ok_or_else(|| invalid("index count overflowed"))?;
    let count = checked_count(count, 2, reader.remaining())?;
    (0..count).map(|_| reader.read_u16_le()).collect()
}

fn read_transparency_keys(
    reader: &mut SliceReader<'_>,
) -> Result<Vec<TransparencyKey>, ContentError> {
    let count = checked_count(reader.read_u32_le()?, 16, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = reader.read_i32_le()?;
        let alpha = reader.read_f32_le()?;
        reader.skip(8)?;
        keys.push(TransparencyKey { frame, alpha });
    }
    Ok(keys)
}

fn read_draw_keys(reader: &mut SliceReader<'_>) -> Result<Vec<DrawKey>, ContentError> {
    let count = checked_count(reader.read_u32_le()?, 16, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = reader.read_i32_le()?;
        reader.skip(4)?;
        let visible = reader.read_i32_le()? != 0;
        reader.skip(4)?;
        keys.push(DrawKey { frame, visible });
    }
    Ok(keys)
}

fn read_texture_keys(reader: &mut SliceReader<'_>) -> Result<Vec<TextureKey>, ContentError> {
    let count = checked_count(reader.read_u32_le()?, 16, reader.remaining())?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = reader.read_i32_le()?;
        reader.skip(8)?;
        let texture = reader.read_i32_le()?;
        keys.push(TextureKey { frame, texture });
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c3::{C3Motion, MotionKey};

    #[test]
    fn transformed_vertex_keeps_the_stock_first_positive_influence_rule() {
        let vertex = C3Vertex {
            position: Vector3::default(),
            texture_coordinate: Vector2::default(),
            colour: 0,
            bone_indices: [0, 1],
            bone_weights: [0.25, 0.75],
        };
        let mesh = C3Mesh {
            name: "test".to_owned(),
            blend_count: 0,
            opaque_vertex_count: 1,
            alpha_vertex_count: 0,
            vertices: vec![vertex],
            opaque_indices: Vec::new(),
            alpha_indices: Vec::new(),
            texture_name: String::new(),
            bounds_minimum: Vector3::default(),
            bounds_maximum: Vector3::default(),
            initial_matrix: Matrix4::IDENTITY,
            texture_rows: 1,
            transparency_keys: Vec::new(),
            draw_keys: Vec::new(),
            texture_keys: Vec::new(),
            extension_tag: None,
        };
        let mut first = Matrix4::IDENTITY;
        first.values[12] = 4.0;
        let mut second = Matrix4::IDENTITY;
        second.values[12] = 40.0;
        let motion = C3Motion {
            bone_count: 2,
            frame_count: 1,
            keys: vec![MotionKey {
                frame: 0,
                bone_matrices: vec![first, second],
            }],
            morph_count: 0,
            morph_values: Vec::new(),
        };

        let result =
            mesh.transformed_vertex_position(&mesh.vertices[0], &motion, Matrix4::IDENTITY, 0.0);
        assert_eq!(result.x, 4.0);
    }
}
