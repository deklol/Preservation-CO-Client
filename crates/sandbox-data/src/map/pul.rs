// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{checked_area, fixed_string, invalid, virtual_path};
use crate::{ContentError, SliceReader, VirtualPath};

#[must_use]
pub const fn is_supported_5065_pux_material(parameters: [u32; 5]) -> bool {
    parameters[0] == 5 && parameters[1] == 6 && parameters[2] == 4
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PuzzleTextureGroup {
    pub tile_id: i16,
    pub label: Vec<u8>,
    pub ani_path: VirtualPath,
    pub section: String,
    pub parameters: [u32; 5],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PuzzleCellLayer {
    pub tile_id: i16,
    pub coverage_mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PuzzleEdgeRecord {
    pub group: u32,
    pub sides: [u8; 4],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PuzzleLayout {
    pub kind: String,
    pub ani_path: VirtualPath,
    pub columns: u32,
    pub rows: u32,
    pub tiles: Vec<i16>,

    pub cell_layers: Vec<Vec<PuzzleCellLayer>>,
    pub texture_groups: Vec<PuzzleTextureGroup>,
    pub edge_groups: Vec<PuzzleTextureGroup>,
    pub edge_records: Vec<PuzzleEdgeRecord>,

    pub roll_speed: [i32; 2],
    pub trailing_bytes: Vec<u8>,
}

impl PuzzleLayout {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        if bytes.starts_with(b"TqTerrain") {
            return super::pux::parse(bytes);
        }
        let mut reader = SliceReader::new("PUL", bytes);
        let kind = fixed_string(&mut reader, 8)?;
        if !kind.starts_with("PUZZLE") {
            return Err(invalid(format!("unexpected PUL kind {kind:?}")));
        }
        let ani = fixed_string(&mut reader, 256)?;
        let columns = u32::try_from(reader.read_i32_le()?)
            .map_err(|_| invalid("negative PUL column count"))?;
        let rows =
            u32::try_from(reader.read_i32_le()?).map_err(|_| invalid("negative PUL row count"))?;
        let count = checked_area(columns, rows, "PUL")?;
        let mut tiles = Vec::with_capacity(count);
        for _ in 0..count {
            tiles.push(reader.read_i16_le()?);
        }
        let roll_speed = if reader.remaining() >= 8 {
            [reader.read_i32_le()?, reader.read_i32_le()?]
        } else {
            [0, 0]
        };
        let trailing_bytes = reader.read_exact(reader.remaining())?.to_vec();
        Ok(Self {
            kind,
            ani_path: virtual_path(&ani, "PUL")?,
            columns,
            rows,
            tiles,
            cell_layers: Vec::new(),
            texture_groups: Vec::new(),
            edge_groups: Vec::new(),
            edge_records: Vec::new(),
            roll_speed,
            trailing_bytes,
        })
    }

    #[must_use]
    pub fn is_layered(&self) -> bool {
        !self.cell_layers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_tile_grid_larger_than_the_payload() {
        let mut bytes = [0_u8; 272];
        bytes[..7].copy_from_slice(b"PUZZLE2");
        bytes[8..24].copy_from_slice(b"ani/newplain.ani");
        bytes[264..268].copy_from_slice(&2_i32.to_le_bytes());
        bytes[268..272].copy_from_slice(&2_i32.to_le_bytes());
        assert!(PuzzleLayout::parse(&bytes).is_err());
    }
}
