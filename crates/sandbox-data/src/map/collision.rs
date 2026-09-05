// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{DMap, MapCell, MapScene};

pub fn apply_fixed_scene_layers(dmap: &mut DMap, scene_x: i32, scene_y: i32, scene: &MapScene) {
    for part in &scene.parts {
        let part_x = scene_x + part.scene_offset_x;
        let part_y = scene_y + part.scene_offset_y;
        for row in 0..part.height {
            for column in 0..part.width {
                let Some(layer) = part.cells.get((row * part.width + column) as usize) else {
                    continue;
                };
                let x = part_x - column as i32;
                let y = part_y - row as i32;
                let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
                    continue;
                };
                if x >= dmap.width || y >= dmap.height {
                    continue;
                }
                let index = y as usize * dmap.width as usize + x as usize;
                let previous = dmap.cells[index];
                dmap.cells[index] = MapCell {
                    access: layer.mask as i16,
                    surface: layer.terrain as i16,
                    elevation: previous.elevation.wrapping_add(layer.altitude as i16),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VirtualPath;
    use crate::map::{MapSceneCell, MapSceneObject, MapScenePart};

    fn map(width: u32, height: u32, cell: MapCell) -> DMap {
        DMap {
            header: [0, 0],
            puzzle_path: VirtualPath::try_from("map/puzzle/test.pul").unwrap(),
            width,
            height,
            cells: vec![cell; (width * height) as usize],
            row_checksums: vec![0; height as usize],
            portals: Vec::new(),
            scene_objects: Vec::<MapSceneObject>::new(),
            terrain_objects: Vec::new(),
            effect_objects: Vec::new(),
            sound_objects: Vec::new(),
            transformed_effect_objects: Vec::new(),
            additional_layer_prefix: None,
            additional_puzzle_layers: Vec::new(),
            trailing_bytes: Vec::new(),
        }
    }

    fn part(width: u32, height: u32, cells: Vec<MapSceneCell>) -> MapScenePart {
        MapScenePart {
            ani_path: VirtualPath::try_from("ani/test.ani").unwrap(),
            section: "Bridge".into(),
            offset_x: 0,
            offset_y: 0,
            frame_interval_ms: 100,
            width,
            height,
            thickness: 0,
            scene_offset_x: 2,
            scene_offset_y: -1,
            elevation: 987_654_321,
            cells,
        }
    }

    #[test]
    fn scene_footprint_uses_reversed_axes_and_replaces_the_effective_layer() {
        let mut map = map(
            8,
            8,
            MapCell {
                access: 1,
                surface: 4,
                elevation: 30,
            },
        );
        let scene = MapScene {
            parts: vec![part(
                2,
                2,
                vec![
                    MapSceneCell {
                        mask: 0,
                        terrain: 8,
                        altitude: 3,
                    },
                    MapSceneCell {
                        mask: 1,
                        terrain: 9,
                        altitude: -4,
                    },
                    MapSceneCell {
                        mask: 2,
                        terrain: 10,
                        altitude: 5,
                    },
                    MapSceneCell {
                        mask: 0,
                        terrain: 11,
                        altitude: 6,
                    },
                ],
            )],
        };

        apply_fixed_scene_layers(&mut map, 3, 4, &scene);

        assert_eq!(
            map.cell(5, 3),
            Some(MapCell {
                access: 0,
                surface: 8,
                elevation: 33,
            })
        );
        assert_eq!(map.cell(4, 3).unwrap().access, 1);
        assert_eq!(map.cell(5, 2).unwrap().access, 2);
        assert_eq!(map.cell(4, 2).unwrap().surface, 11);
        assert_eq!(map.cell(4, 2).unwrap().elevation, 36);
    }

    #[test]
    fn later_scene_layers_add_to_the_current_top_layer() {
        let mut map = map(
            2,
            2,
            MapCell {
                access: 1,
                surface: 0,
                elevation: i16::MAX,
            },
        );
        let scene = MapScene {
            parts: vec![part(
                1,
                1,
                vec![MapSceneCell {
                    mask: 0,
                    terrain: 7,
                    altitude: 1,
                }],
            )],
        };

        apply_fixed_scene_layers(&mut map, -2, 2, &scene);

        assert_eq!(
            map.cell(0, 1),
            Some(MapCell {
                access: 0,
                surface: 7,
                elevation: i16::MIN,
            })
        );
    }

    #[test]
    fn cells_outside_the_map_are_ignored() {
        let original = MapCell {
            access: 1,
            surface: 2,
            elevation: 3,
        };
        let mut map = map(1, 1, original);
        let scene = MapScene {
            parts: vec![part(
                1,
                1,
                vec![MapSceneCell {
                    mask: 0,
                    terrain: 0,
                    altitude: 0,
                }],
            )],
        };

        apply_fixed_scene_layers(&mut map, -100, -100, &scene);

        assert_eq!(map.cell(0, 0), Some(original));
    }
}
