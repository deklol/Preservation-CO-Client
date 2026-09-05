// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{
    MAXIMUM_MAP_OBJECTS, checked_area, checked_count, fixed_string, invalid, virtual_path,
};
use crate::{ContentError, SliceReader, VirtualPath};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MapCell {
    pub access: i16,
    pub surface: i16,
    pub elevation: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapPortal {
    pub x: i32,
    pub y: i32,
    pub index: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerrainObject {
    pub ani_path: VirtualPath,
    pub section: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub frame_interval_ms: u32,
}

impl TerrainObject {
    #[must_use]
    pub const fn is_renderable(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapSceneObject {
    pub scene_path: VirtualPath,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapEffectObject {
    pub effect: String,
    pub world_x: i32,
    pub world_y: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapSoundObject {
    pub sound_path: VirtualPath,
    pub world_x: i32,
    pub world_y: i32,
    pub range: u32,
    pub volume: u32,
    pub interval_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapTransformedEffectObject {
    pub effect: String,
    pub world_x: i32,
    pub world_y: i32,
    pub transform_bits: [u32; 6],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapEffectTransform {
    pub vertical_radians: f32,
    pub horizontal_radians: f32,
    pub scale: [f32; 3],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapAdditionalPuzzleLayer {
    pub insertion_index: u32,
    pub move_rate_x: i32,
    pub move_rate_y: i32,
    pub state_words: [u32; 3],
    pub puzzle_paths: Vec<VirtualPath>,
}

impl MapEffectObject {
    #[must_use]
    pub fn map_position(&self, map_width: u32) -> [f32; 2] {
        effect_map_position(self.world_x, self.world_y, map_width)
    }
}

impl MapTransformedEffectObject {
    #[must_use]
    pub fn map_position(&self, map_width: u32) -> [f32; 2] {
        effect_map_position(self.world_x, self.world_y, map_width)
    }

    #[must_use]
    pub fn transform(&self) -> MapEffectTransform {
        MapEffectTransform {
            vertical_radians: f32::from_bits(self.transform_bits[1]),
            horizontal_radians: f32::from_bits(self.transform_bits[2]),
            scale: [
                f32::from_bits(self.transform_bits[3]),
                f32::from_bits(self.transform_bits[4]),
                f32::from_bits(self.transform_bits[5]),
            ],
        }
    }
}

fn effect_map_position(world_x: i32, world_y: i32, map_width: u32) -> [f32; 2] {
    let difference = (world_x as f32 - map_width as f32 * 32.0) / 64.0;
    let sum = (world_y as f32 - 16.0) / 32.0;
    [difference + sum, sum - difference]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DMap {
    pub header: [u32; 2],
    pub puzzle_path: VirtualPath,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<MapCell>,
    pub row_checksums: Vec<u32>,
    pub portals: Vec<MapPortal>,
    pub scene_objects: Vec<MapSceneObject>,
    pub terrain_objects: Vec<TerrainObject>,
    pub effect_objects: Vec<MapEffectObject>,
    pub sound_objects: Vec<MapSoundObject>,
    pub transformed_effect_objects: Vec<MapTransformedEffectObject>,

    pub additional_layer_prefix: Option<u32>,
    pub additional_puzzle_layers: Vec<MapAdditionalPuzzleLayer>,
    pub trailing_bytes: Vec<u8>,
}

impl DMap {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("DMap", bytes);
        let header = [reader.read_u32_le()?, reader.read_u32_le()?];
        let puzzle = fixed_string(&mut reader, 260)?;
        let width = reader.read_u32_le()?;
        let height = reader.read_u32_le()?;
        let cell_count = checked_area(width, height, "DMap")?;
        let mut cells = Vec::with_capacity(cell_count);
        let mut row_checksums = Vec::with_capacity(height as usize);
        for _ in 0..height {
            for _ in 0..width {
                cells.push(MapCell {
                    access: reader.read_i16_le()?,
                    surface: reader.read_i16_le()?,
                    elevation: reader.read_i16_le()?,
                });
            }
            row_checksums.push(reader.read_u32_le()?);
        }

        let portal_count =
            checked_count(reader.read_i32_le()?, MAXIMUM_MAP_OBJECTS, "portal count")?;
        let mut portals = Vec::with_capacity(portal_count as usize);
        for _ in 0..portal_count {
            portals.push(MapPortal {
                x: reader.read_i32_le()?,
                y: reader.read_i32_le()?,
                index: reader.read_i32_le()?,
            });
        }

        let object_count =
            checked_count(reader.read_i32_le()?, MAXIMUM_MAP_OBJECTS, "object count")?;
        let mut scene_objects = Vec::new();
        let mut terrain_objects = Vec::new();
        let mut effect_objects = Vec::new();
        let mut sound_objects = Vec::new();
        let mut transformed_effect_objects = Vec::new();

        let extended_object_layout = header[0] == 1005;
        for index in 0..object_count {
            let mut object_type = reader.read_i32_le()?;
            if object_type == 0 {
                object_type = reader.read_i32_le()?;
            }
            match object_type {
                1 => scene_objects.push(MapSceneObject {
                    scene_path: virtual_path(&fixed_string(&mut reader, 260)?, "DMap scene")?,
                    x: reader.read_i32_le()?,
                    y: reader.read_i32_le()?,
                }),
                4 | 24 => terrain_objects.push(TerrainObject {
                    ani_path: virtual_path(&fixed_string(&mut reader, 260)?, "DMap terrain")?,
                    section: fixed_string(&mut reader, 128)?,
                    x: reader.read_i32_le()?,
                    y: reader.read_i32_le()?,
                    width: reader.read_i32_le()?,
                    height: reader.read_i32_le()?,
                    offset_x: reader.read_i32_le()?,
                    offset_y: reader.read_i32_le()?,
                    frame_interval_ms: reader.read_u32_le()?,
                }),
                10 => effect_objects.push(MapEffectObject {
                    effect: fixed_string(&mut reader, 64)?,
                    world_x: reader.read_i32_le()?,
                    world_y: reader.read_i32_le()?,
                }),

                19 if extended_object_layout => {
                    transformed_effect_objects.push(MapTransformedEffectObject {
                        effect: fixed_string(&mut reader, 64)?,
                        world_x: reader.read_i32_le()?,
                        world_y: reader.read_i32_le()?,
                        transform_bits: [
                            reader.read_u32_le()?,
                            reader.read_u32_le()?,
                            reader.read_u32_le()?,
                            reader.read_u32_le()?,
                            reader.read_u32_le()?,
                            reader.read_u32_le()?,
                        ],
                    });
                }
                19 => reader.skip(72)?,

                15 if extended_object_layout => sound_objects.push(MapSoundObject {
                    sound_path: virtual_path(&fixed_string(&mut reader, 260)?, "DMap sound")?,
                    world_x: reader.read_i32_le()?,
                    world_y: reader.read_i32_le()?,
                    range: reader.read_u32_le()?,
                    volume: reader.read_u32_le()?,
                    interval_ms: reader.read_u32_le()?,
                }),
                15 => reader.skip(276)?,
                _ => {
                    return Err(invalid(format!(
                        "unsupported DMap object type {object_type} at index {index}"
                    )));
                }
            }
        }
        let (additional_layer_prefix, additional_puzzle_layers) = if extended_object_layout
            && reader.remaining() != 0
        {
            if reader.remaining() < 8 {
                return Err(invalid("truncated version-1005 additional-layer header"));
            }
            let prefix = reader.read_u32_le()?;
            let layer_count = reader.read_u32_le()?;
            if layer_count > MAXIMUM_MAP_OBJECTS {
                return Err(invalid(format!(
                    "version-1005 additional-layer count {layer_count} is unreasonable"
                )));
            }
            let mut layers = Vec::with_capacity(layer_count as usize);
            for layer_index in 0..layer_count {
                let insertion_index = reader.read_u32_le()?;
                let layer_type = reader.read_u32_le()?;
                if layer_type != 4 {
                    return Err(invalid(format!(
                        "unsupported version-1005 scene layer type {layer_type} at index {layer_index}"
                    )));
                }
                let move_rate_x = reader.read_i32_le()?;
                let move_rate_y = reader.read_i32_le()?;
                let state_words = [
                    reader.read_u32_le()?,
                    reader.read_u32_le()?,
                    reader.read_u32_le()?,
                ];
                let object_count = reader.read_u32_le()?;
                if object_count > MAXIMUM_MAP_OBJECTS {
                    return Err(invalid(format!(
                        "version-1005 scene layer {layer_index} object count {object_count} is unreasonable"
                    )));
                }
                let mut puzzle_paths = Vec::with_capacity(object_count as usize);
                for object_index in 0..object_count {
                    let object_type = reader.read_u32_le()?;
                    if object_type != 8 {
                        return Err(invalid(format!(
                            "unsupported version-1005 scene-layer object type {object_type} at layer {layer_index} object {object_index}"
                        )));
                    }
                    puzzle_paths.push(virtual_path(
                        &fixed_string(&mut reader, 260)?,
                        "DMap additional puzzle",
                    )?);
                }
                layers.push(MapAdditionalPuzzleLayer {
                    insertion_index,
                    move_rate_x,
                    move_rate_y,
                    state_words,
                    puzzle_paths,
                });
            }
            (Some(prefix), layers)
        } else {
            (None, Vec::new())
        };
        let trailing_bytes = reader.read_exact(reader.remaining())?.to_vec();
        Ok(Self {
            header,
            puzzle_path: virtual_path(&puzzle, "DMap")?,
            width,
            height,
            cells,
            row_checksums,
            portals,
            scene_objects,
            terrain_objects,
            effect_objects,
            sound_objects,
            transformed_effect_objects,
            additional_layer_prefix,
            additional_puzzle_layers,
            trailing_bytes,
        })
    }

    pub fn cell(&self, x: u32, y: u32) -> Option<MapCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = usize::try_from(y)
            .ok()?
            .checked_mul(usize::try_from(self.width).ok()?)?
            .checked_add(usize::try_from(x).ok()?)?;
        self.cells.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{DMap, MapAdditionalPuzzleLayer, MapEffectObject, MapEffectTransform};

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_fixed_string(bytes: &mut Vec<u8>, value: &str, width: usize) {
        assert!(value.len() < width);
        bytes.extend_from_slice(value.as_bytes());
        bytes.resize(bytes.len() + width - value.len(), 0);
    }

    #[test]
    fn preserves_exact_sound_alignment_and_classifies_inert_covers() {
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, 0);
        push_fixed_string(&mut bytes, "map/puzzle/test.pul", 260);
        push_u32(&mut bytes, 1);
        push_u32(&mut bytes, 1);
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        push_u32(&mut bytes, 0);
        push_i32(&mut bytes, 0);
        push_i32(&mut bytes, 4);

        push_i32(&mut bytes, 4);
        push_fixed_string(&mut bytes, "AniTitle=", 260);
        push_fixed_string(&mut bytes, "PosCell=[0,0]", 128);
        for value in [0, 0, 0, 0, 0, 0] {
            push_i32(&mut bytes, value);
        }
        push_u32(&mut bytes, 0);

        push_i32(&mut bytes, 15);
        bytes.resize(bytes.len() + 276, 0);

        push_i32(&mut bytes, 10);
        push_fixed_string(&mut bytes, "wave", 64);
        push_i32(&mut bytes, 12_266);
        push_i32(&mut bytes, 6_230);

        push_i32(&mut bytes, 4);
        push_fixed_string(&mut bytes, "ani/sand.ani", 260);
        push_fixed_string(&mut bytes, "PosCell=[12,34]", 128);
        for value in [12, 34, 2, 3, -4, 5] {
            push_i32(&mut bytes, value);
        }
        push_u32(&mut bytes, 125);

        let map = DMap::parse(&bytes).expect("parse aligned DMap objects");
        assert_eq!(map.terrain_objects.len(), 2);
        assert!(!map.terrain_objects[0].is_renderable());
        let terrain = &map.terrain_objects[1];
        assert_eq!(terrain.ani_path.as_str(), "ani/sand.ani");
        assert_eq!(terrain.section, "PosCell=[12,34]");
        assert_eq!((terrain.x, terrain.y), (12, 34));
        assert_eq!((terrain.width, terrain.height), (2, 3));
        assert_eq!((terrain.offset_x, terrain.offset_y), (-4, 5));
        assert_eq!(terrain.frame_interval_ms, 125);
        assert!(terrain.is_renderable());
        assert_eq!(
            map.effect_objects,
            vec![MapEffectObject {
                effect: "wave".to_owned(),
                world_x: 12_266,
                world_y: 6_230,
            }]
        );
    }

    #[test]
    fn preserves_fractional_market_effect_placement() {
        let effect = MapEffectObject {
            effect: "wave".to_owned(),
            world_x: 12_266,
            world_y: 6_230,
        };

        assert_eq!(effect.map_position(384), [193.84375, 194.53125]);
    }

    #[test]
    fn parses_extended_sound_and_transformed_effect_records_without_changing_classic_widths() {
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 1005);
        push_u32(&mut bytes, 0);
        push_fixed_string(&mut bytes, "PuzzleSave/test.pux", 260);
        push_u32(&mut bytes, 1);
        push_u32(&mut bytes, 1);
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        push_u32(&mut bytes, 0);
        push_i32(&mut bytes, 0);
        push_i32(&mut bytes, 2);

        push_i32(&mut bytes, 15);
        push_fixed_string(&mut bytes, "sound/river.wav", 260);
        for value in [320_i32, 640] {
            push_i32(&mut bytes, value);
        }
        for value in [18_u32, 75, 2_000] {
            push_u32(&mut bytes, value);
        }

        push_i32(&mut bytes, 19);
        push_fixed_string(&mut bytes, "waterfall", 64);
        push_i32(&mut bytes, 960);
        push_i32(&mut bytes, 1_280);
        for value in [123.0_f32, 0.25, 5.5, 1.0, 2.0, 3.0] {
            push_u32(&mut bytes, value.to_bits());
        }

        let map = DMap::parse(&bytes).expect("parse extended objects");
        assert_eq!(map.sound_objects.len(), 1);
        assert_eq!(map.sound_objects[0].sound_path.as_str(), "sound/river.wav");
        assert_eq!(
            (map.sound_objects[0].world_x, map.sound_objects[0].world_y),
            (320, 640)
        );
        assert_eq!(
            (map.sound_objects[0].range, map.sound_objects[0].volume),
            (18, 75)
        );
        assert_eq!(map.sound_objects[0].interval_ms, 2_000);
        assert_eq!(map.transformed_effect_objects.len(), 1);
        let transformed = &map.transformed_effect_objects[0];
        assert_eq!(transformed.effect, "waterfall");
        assert_eq!(
            transformed.transform_bits,
            [123.0_f32, 0.25, 5.5, 1.0, 2.0, 3.0].map(f32::to_bits)
        );
        assert_eq!(
            transformed.transform(),
            MapEffectTransform {
                vertical_radians: 0.25,
                horizontal_radians: 5.5,
                scale: [1.0, 2.0, 3.0],
            }
        );
        assert_eq!(transformed.map_position(1), [54.0, 25.0]);
        assert!(map.additional_puzzle_layers.is_empty());
    }

    #[test]
    fn parses_version_1005_additional_puzzle_layers_and_retains_suffix() {
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 1005);
        push_u32(&mut bytes, 0);
        push_fixed_string(&mut bytes, "PuzzleSave/test.pux", 260);
        push_u32(&mut bytes, 1);
        push_u32(&mut bytes, 1);
        for _ in 0..3 {
            bytes.extend_from_slice(&0_i16.to_le_bytes());
        }
        push_u32(&mut bytes, 0);
        push_i32(&mut bytes, 0);
        push_i32(&mut bytes, 0);

        push_u32(&mut bytes, 0x1122_3344);
        push_u32(&mut bytes, 1);
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, 4);
        push_i32(&mut bytes, 30);
        push_i32(&mut bytes, 30);
        for value in [160, 5, 6, 1, 8] {
            push_u32(&mut bytes, value);
        }
        push_fixed_string(&mut bytes, "map/puzzle/background.pul", 260);
        bytes.extend_from_slice(&[0xaa; 8]);

        let map = DMap::parse(&bytes).expect("parse version-1005 scene layer");
        assert_eq!(map.additional_layer_prefix, Some(0x1122_3344));
        assert_eq!(
            map.additional_puzzle_layers,
            vec![MapAdditionalPuzzleLayer {
                insertion_index: 0,
                move_rate_x: 30,
                move_rate_y: 30,
                state_words: [160, 5, 6],
                puzzle_paths: vec!["map/puzzle/background.pul".try_into().unwrap()],
            }]
        );
        assert_eq!(map.trailing_bytes, [0xaa; 8]);
    }
}
