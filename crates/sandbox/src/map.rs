// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::assets::{Assets, Result};
use sandbox_data::{
    ini::SectionCatalog,
    map::{DMap, GameMapCatalog, MapScene, PuzzleLayout, apply_fixed_scene_layers},
};

pub struct Sprite {
    pub frames: Vec<String>,
    pub origin: [f32; 2],
    pub depth: f32,
    pub interval: u32,
}
pub struct Map {
    pub id: u32,
    pub data: DMap,
    pub sprites: Vec<Sprite>,
    pub extent: [f32; 2],
}
impl Map {
    pub fn load(assets: &mut Assets, id: u32) -> Result<Self> {
        let catalog = GameMapCatalog::parse(&assets.read("ini/GameMap.dat")?)?;
        let record = catalog.get(id).ok_or("Map is not in GameMap.dat")?;
        let mut data = DMap::parse(&assets.read(record.dmap_path.as_str())?)?;
        let puzzle = PuzzleLayout::parse(&assets.read(data.puzzle_path.as_str())?)?;
        if !puzzle.cell_layers.is_empty() || !data.additional_puzzle_layers.is_empty() {
            return Err(
                "This minimal renderer supports flat PUL maps, not layered PUX maps".into(),
            );
        }
        let tile = record.tile_size as f32;
        if tile <= 0.0 {
            return Err("invalid map tile size".into());
        }
        let extent = [puzzle.columns as f32 * tile, puzzle.rows as f32 * tile];
        let mut sprites = Vec::new();
        let ani = SectionCatalog::parse(&assets.read(puzzle.ani_path.as_str())?);
        for (i, id) in puzzle.tiles.iter().enumerate() {
            if *id < 0 {
                continue;
            }
            let frames = frames(&ani, &format!("Puzzle{id}"))?;
            sprites.push(Sprite {
                frames,
                origin: [
                    (i as u32 % puzzle.columns) as f32 * tile,
                    (i as u32 / puzzle.columns) as f32 * tile,
                ],
                depth: f32::NEG_INFINITY,
                interval: 100,
            });
        }
        let project = |x: f32, y: f32| {
            [
                (x - y) * 32.0 + extent[0] * 0.5,
                (x + y - (data.height - 1) as f32) * 16.0 + extent[1] * 0.5,
            ]
        };
        for object in &data.terrain_objects {
            if !object.is_renderable() {
                continue;
            }
            let ani = SectionCatalog::parse(&assets.read(object.ani_path.as_str())?);
            let p = project(object.x as f32, object.y as f32);
            sprites.push(Sprite {
                frames: frames(&ani, &object.section)?,
                origin: [p[0] - object.offset_x as f32, p[1] - object.offset_y as f32],
                depth: (object.x + object.y) as f32,
                interval: object.frame_interval_ms.max(1),
            });
        }
        let scene_objects = data.scene_objects.clone();
        for object in scene_objects {
            let scene = MapScene::parse(&assets.read(object.scene_path.as_str())?)?;
            for part in &scene.parts {
                let x = object.x + part.scene_offset_x;
                let y = object.y + part.scene_offset_y;
                let p = [
                    (x - y) as f32 * 32.0 + extent[0] * 0.5,
                    (x + y - (data.height - 1) as i32) as f32 * 16.0 + extent[1] * 0.5,
                ];
                let ani = SectionCatalog::parse(&assets.read(part.ani_path.as_str())?);
                sprites.push(Sprite {
                    frames: frames(&ani, &part.section)?,
                    origin: [p[0] + part.offset_x as f32, p[1] + part.offset_y as f32],
                    depth: (x + y) as f32,
                    interval: part.frame_interval_ms.max(1),
                });
            }
            apply_fixed_scene_layers(&mut data, object.x, object.y, &scene);
        }

        sprites.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        Ok(Self {
            id,
            data,
            sprites,
            extent,
        })
    }
    pub fn project(&self, p: [f32; 2]) -> [f32; 2] {
        [
            (p[0] - p[1]) * 32.0 + self.extent[0] * 0.5,
            (p[0] + p[1] - (self.data.height - 1) as f32) * 16.0 + self.extent[1] * 0.5,
        ]
    }
    pub fn unproject(&self, p: [f32; 2]) -> [f32; 2] {
        let d = (p[0] - self.extent[0] * 0.5) / 32.0;
        let s = (p[1] - self.extent[1] * 0.5) / 16.0 + (self.data.height - 1) as f32;
        [(s + d) * 0.5, (s - d) * 0.5]
    }
    pub fn walkable(&self, p: [i32; 2]) -> bool {
        p[0] >= 0
            && p[1] >= 0
            && self
                .data
                .cell(p[0] as u32, p[1] as u32)
                .is_some_and(|c| c.access == 0)
    }
}
fn frames(ani: &SectionCatalog, name: &str) -> Result<Vec<String>> {
    let section = ani
        .section(name)
        .ok_or_else(|| format!("Missing ANI section {name}"))?;
    let count = section
        .first("FrameAmount")
        .unwrap_or("1")
        .parse::<usize>()?;
    if count == 0 || count > 4096 {
        return Err("invalid ANI frame count".into());
    }
    (0..count)
        .map(|i| {
            section
                .first(&format!("Frame{i}"))
                .map(|s| s.replace('\\', "/"))
                .ok_or_else(|| format!("Missing {name}/Frame{i}").into())
        })
        .collect()
}
