// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{
    MAXIMUM_MAP_OBJECTS, PuzzleCellLayer, PuzzleEdgeRecord, PuzzleLayout, PuzzleTextureGroup,
    checked_area, invalid, virtual_path,
};
use crate::{ContentError, SliceReader};
use std::collections::HashSet;

const SIGNATURE_LENGTH: usize = 16;
const TERRAIN_TOKEN: u32 = 1000;
const COVERAGE_MASK: u32 = 0x01ff_ffff;

pub(super) fn parse(bytes: &[u8]) -> Result<PuzzleLayout, ContentError> {
    let mut reader = SliceReader::new("PUX", bytes);
    let kind = nul_terminated_ascii(reader.read_exact(SIGNATURE_LENGTH)?, "PUX signature")?;
    if kind != "TqTerrain" {
        return Err(invalid(format!("unexpected PUX kind {kind:?}")));
    }
    expect_token(&mut reader, "PUX header")?;
    let columns = reader.read_u32_le()?;
    let rows = reader.read_u32_le()?;
    let cell_count = checked_area(columns, rows, "PUX")?;

    expect_token(&mut reader, "PUX texture groups")?;
    let texture_groups = read_groups(&mut reader, "PUX texture group")?;
    expect_token(&mut reader, "PUX edge groups")?;
    let edge_groups = read_groups(&mut reader, "PUX edge group")?;

    let declared_cells = usize::try_from(reader.read_u32_le()?)
        .map_err(|_| invalid("PUX cell count does not fit this platform"))?;
    if declared_cells != cell_count {
        return Err(invalid(format!(
            "PUX declares {declared_cells} cells for a {columns}x{rows} grid"
        )));
    }

    let texture_ids = texture_groups
        .iter()
        .map(|group| group.tile_id)
        .collect::<HashSet<_>>();
    let mut tiles = Vec::with_capacity(cell_count);
    let mut cell_layers = Vec::with_capacity(cell_count);
    for cell_index in 0..cell_count {
        let layer_count = usize::from(reader.read_exact(1)?[0]);
        let mut layers = Vec::with_capacity(layer_count);
        let mut assigned_vertices = 0_u32;
        for layer_index in 0..layer_count {
            let raw_tile_id = reader.read_u16_le()?;
            let tile_id = i16::try_from(raw_tile_id).map_err(|_| {
                invalid(format!(
                    "PUX cell {cell_index} layer {layer_index} tile ID {raw_tile_id} exceeds the shared tile range"
                ))
            })?;
            if !texture_ids.contains(&tile_id) {
                return Err(invalid(format!(
                    "PUX cell {cell_index} layer {layer_index} references missing texture group {tile_id}"
                )));
            }
            let coverage_mask = reader.read_u32_le()?;
            if coverage_mask & !COVERAGE_MASK != 0 {
                return Err(invalid(format!(
                    "PUX cell {cell_index} layer {layer_index} has coverage bits outside the 5x5 terrain mask: {coverage_mask:#010x}"
                )));
            }
            let overlap = assigned_vertices & coverage_mask;
            if overlap != 0 {
                return Err(invalid(format!(
                    "PUX cell {cell_index} layer {layer_index} overlaps earlier coverage vertices: {overlap:#010x}"
                )));
            }
            assigned_vertices |= coverage_mask;
            layers.push(PuzzleCellLayer {
                tile_id,
                coverage_mask,
            });
        }
        tiles.push(layers.first().map_or(-1, |layer| layer.tile_id));
        cell_layers.push(layers);
    }

    let edge_count = usize::try_from(reader.read_u32_le()?)
        .ok()
        .filter(|count| *count <= MAXIMUM_MAP_OBJECTS as usize)
        .ok_or_else(|| invalid("PUX edge record count is unreasonable"))?;
    let mut edge_records = Vec::with_capacity(edge_count);
    for _ in 0..edge_count {
        edge_records.push(PuzzleEdgeRecord {
            group: reader.read_u32_le()?,
            sides: reader
                .read_exact(4)?
                .try_into()
                .expect("edge side length was checked"),
        });
    }
    if reader.remaining() != 0 {
        return Err(invalid(format!(
            "PUX has {} unexplained trailing bytes",
            reader.remaining()
        )));
    }

    let ani_path = texture_groups
        .first()
        .map(|group| group.ani_path.clone())
        .ok_or_else(|| invalid("PUX declares no texture groups"))?;
    Ok(PuzzleLayout {
        kind,
        ani_path,
        columns,
        rows,
        tiles,
        cell_layers,
        texture_groups,
        edge_groups,
        edge_records,
        roll_speed: [0, 0],
        trailing_bytes: Vec::new(),
    })
}

fn expect_token(reader: &mut SliceReader<'_>, owner: &str) -> Result<(), ContentError> {
    let token = reader.read_u32_le()?;
    if token != TERRAIN_TOKEN {
        return Err(invalid(format!(
            "{owner} token is {token}, expected {TERRAIN_TOKEN}"
        )));
    }
    Ok(())
}

fn read_groups(
    reader: &mut SliceReader<'_>,
    owner: &'static str,
) -> Result<Vec<PuzzleTextureGroup>, ContentError> {
    let count = usize::from(reader.read_u16_le()?);
    let mut groups = Vec::with_capacity(count);
    let mut ids = HashSet::with_capacity(count);
    for index in 0..count {
        let label_length = usize::from(reader.read_u16_le()?);
        let label = reader.read_exact(label_length)?.to_vec();
        let ani = sized_ascii(reader, owner)?;
        let section = sized_ascii(reader, owner)?;
        let tile_id = puzzle_section_id(&section).ok_or_else(|| {
            invalid(format!(
                "{owner} {index} section {section:?} is not a Puzzle<number> section"
            ))
        })?;
        if !ids.insert(tile_id) {
            return Err(invalid(format!(
                "{owner} tile ID {tile_id} is declared more than once"
            )));
        }
        groups.push(PuzzleTextureGroup {
            tile_id,
            label,
            ani_path: virtual_path(&ani, owner)?,
            section,
            parameters: [
                reader.read_u32_le()?,
                reader.read_u32_le()?,
                reader.read_u32_le()?,
                reader.read_u32_le()?,
                reader.read_u32_le()?,
            ],
        });
    }
    Ok(groups)
}

fn sized_ascii(reader: &mut SliceReader<'_>, owner: &'static str) -> Result<String, ContentError> {
    let length = usize::from(reader.read_u16_le()?);
    nul_terminated_ascii(reader.read_exact(length)?, owner)
}

fn nul_terminated_ascii(bytes: &[u8], owner: &'static str) -> Result<String, ContentError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if !bytes[..end].is_ascii() {
        return Err(invalid(format!("{owner} contains non-ASCII path data")));
    }
    Ok(String::from_utf8_lossy(&bytes[..end]).replace('\\', "/"))
}

fn puzzle_section_id(section: &str) -> Option<i16> {
    section.strip_prefix("Puzzle")?.parse::<i16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_sized(bytes: &mut Vec<u8>, value: &[u8]) {
        bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
        bytes.extend_from_slice(value);
    }

    fn group(bytes: &mut Vec<u8>, section: &str) {
        push_sized(bytes, b"label");
        push_sized(bytes, b"ANI\\MY.ANI");
        push_sized(bytes, section.as_bytes());
        for value in [5_u32, 6, 4, 1, u32::MAX] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn fixture(second_mask: u32) -> Vec<u8> {
        let mut bytes = [b"TqTerrain".as_slice(), &[0_u8; 7]].concat();
        bytes.extend_from_slice(&TERRAIN_TOKEN.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&TERRAIN_TOKEN.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        group(&mut bytes, "Puzzle7");
        group(&mut bytes, "Puzzle8");
        bytes.extend_from_slice(&TERRAIN_TOKEN.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.push(2);
        bytes.extend_from_slice(&7_u16.to_le_bytes());
        bytes.extend_from_slice(&0x0000_000f_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&second_mask.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes
    }

    #[test]
    fn preserves_ordered_layer_masks_and_group_sources() {
        let layout = parse(&fixture(0x01ff_fff0)).expect("parse layered terrain");
        assert_eq!(layout.kind, "TqTerrain");
        assert_eq!((layout.columns, layout.rows), (1, 1));
        assert_eq!(layout.tiles, [7]);
        assert_eq!(layout.texture_groups.len(), 2);
        assert_eq!(layout.texture_groups[0].ani_path.as_str(), "ANI/MY.ANI");
        assert_eq!(layout.texture_groups[0].parameters, [5, 6, 4, 1, u32::MAX]);
        assert_eq!(
            layout.cell_layers[0],
            [
                PuzzleCellLayer {
                    tile_id: 7,
                    coverage_mask: 0x0000_000f,
                },
                PuzzleCellLayer {
                    tile_id: 8,
                    coverage_mask: 0x01ff_fff0,
                },
            ]
        );
    }

    #[test]
    fn rejects_overlapping_layer_coverage() {
        let error = parse(&fixture(0x0000_0010 | 0x0000_0001)).unwrap_err();
        assert!(error.to_string().contains("overlaps earlier coverage"));
    }

    #[test]
    fn rejects_coverage_outside_the_5x5_mask() {
        let error = parse(&fixture(0x0200_0000)).unwrap_err();
        assert!(error.to_string().contains("outside the 5x5 terrain mask"));
    }
}
