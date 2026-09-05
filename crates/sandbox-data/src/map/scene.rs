// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{
    MAXIMUM_MAP_OBJECTS, checked_area, checked_count, fixed_string, invalid, virtual_path,
};
use crate::{ContentError, SliceReader, VirtualPath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapSceneCell {
    pub mask: u32,
    pub terrain: i32,
    pub altitude: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapScenePart {
    pub ani_path: VirtualPath,
    pub section: String,
    pub offset_x: i32,
    pub offset_y: i32,
    pub frame_interval_ms: u32,
    pub width: u32,
    pub height: u32,
    pub thickness: i32,
    pub scene_offset_x: i32,
    pub scene_offset_y: i32,
    pub elevation: i32,
    pub cells: Vec<MapSceneCell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapScene {
    pub parts: Vec<MapScenePart>,
}

impl MapScene {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("map scene", bytes);
        let part_count = checked_count(
            reader.read_i32_le()?,
            MAXIMUM_MAP_OBJECTS,
            "scene part count",
        )?;
        let mut parts = Vec::with_capacity(part_count as usize);
        for _ in 0..part_count {
            let ani_path = virtual_path(&fixed_string(&mut reader, 256)?, "map scene ANI")?;
            let section = fixed_string(&mut reader, 64)?;
            let offset_x = reader.read_i32_le()?;
            let offset_y = reader.read_i32_le()?;
            let frame_interval_ms = reader.read_u32_le()?;
            let width = reader.read_u32_le()?;
            let height = reader.read_u32_le()?;
            let cell_count = checked_area(width, height, "map scene part")?;
            let thickness = reader.read_i32_le()?;
            let scene_offset_x = reader.read_i32_le()?;
            let scene_offset_y = reader.read_i32_le()?;
            let elevation = reader.read_i32_le()?;
            let mut cells = Vec::with_capacity(cell_count);
            for _ in 0..cell_count {
                cells.push(MapSceneCell {
                    mask: reader.read_u32_le()?,
                    terrain: reader.read_i32_le()?,
                    altitude: reader.read_i32_le()?,
                });
            }
            parts.push(MapScenePart {
                ani_path,
                section,
                offset_x,
                offset_y,
                frame_interval_ms,
                width,
                height,
                thickness,
                scene_offset_x,
                scene_offset_y,
                elevation,
                cells,
            });
        }
        if reader.remaining() != 0 {
            return Err(invalid(format!(
                "map scene has {} trailing bytes",
                reader.remaining()
            )));
        }
        Ok(Self { parts })
    }
}
