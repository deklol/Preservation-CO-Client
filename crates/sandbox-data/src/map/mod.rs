// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
mod catalog;
mod collision;
mod dmap;
mod pul;
mod pux;
mod scene;

pub use catalog::{GameMapCatalog, GameMapRecord};
pub use collision::apply_fixed_scene_layers;
pub use dmap::{
    DMap, MapAdditionalPuzzleLayer, MapCell, MapEffectObject, MapEffectTransform, MapPortal,
    MapSceneObject, MapSoundObject, MapTransformedEffectObject, TerrainObject,
};
pub use pul::{
    PuzzleCellLayer, PuzzleEdgeRecord, PuzzleLayout, PuzzleTextureGroup,
    is_supported_5065_pux_material,
};
pub use scene::{MapScene, MapSceneCell, MapScenePart};

use crate::{ContentError, SliceReader, VirtualPath};

const MAXIMUM_MAP_DIMENSION: u32 = 4096;
const MAXIMUM_MAP_RECORDS: u32 = 100_000;
const MAXIMUM_MAP_OBJECTS: u32 = 100_000;

fn fixed_string(reader: &mut SliceReader<'_>, length: usize) -> Result<String, ContentError> {
    let bytes = reader.read_exact(length)?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if !bytes[..end].is_ascii() {
        return Err(invalid("fixed string contains non-ASCII bytes"));
    }
    Ok(String::from_utf8_lossy(&bytes[..end]).replace('\\', "/"))
}

fn virtual_path(value: &str, context: &'static str) -> Result<VirtualPath, ContentError> {
    VirtualPath::try_from(value).map_err(|_| ContentError::InvalidData {
        context,
        detail: format!("invalid virtual path {value:?}"),
    })
}

fn checked_count(value: i32, maximum: u32, detail: &str) -> Result<u32, ContentError> {
    u32::try_from(value)
        .ok()
        .filter(|count| *count <= maximum)
        .ok_or_else(|| invalid(format!("invalid {detail} {value}")))
}

fn checked_area(width: u32, height: u32, context: &'static str) -> Result<usize, ContentError> {
    if width == 0 || height == 0 || width > MAXIMUM_MAP_DIMENSION || height > MAXIMUM_MAP_DIMENSION
    {
        return Err(ContentError::InvalidData {
            context,
            detail: format!("invalid dimensions {width}x{height}"),
        });
    }
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(ContentError::InvalidData {
            context,
            detail: "cell count overflows this platform".to_owned(),
        })
}

fn invalid(detail: impl Into<String>) -> ContentError {
    ContentError::InvalidData {
        context: "game map",
        detail: detail.into(),
    }
}
