// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::{
    assets::{Assets, Result},
    map::Map,
};
use sandbox_data::ini::SectionCatalog;

pub struct Quad {
    pub texture: &'static str,
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

pub struct Minimap {
    pub images: Vec<(&'static str, image::RgbaImage)>,
    size: [f32; 2],
    single: bool,
    fitted: bool,
    expanded: bool,
    pressed: Option<usize>,
}

impl Minimap {
    pub fn load(assets: &mut Assets, map_id: u32) -> Result<Option<Self>> {
        let catalog = SectionCatalog::parse(&assets.read("ani/MiniMap.Ani")?);
        let Some(section) = catalog.section(&map_id.to_string()) else {
            return Ok(None);
        };
        let count: usize = section.first("FrameAmount").unwrap_or("1").parse()?;
        if count == 0 {
            return Ok(None);
        }
        if count != 1 && count != 4 {
            return Err("Unsupported minimap frame count".into());
        }
        let frames = (0..count)
            .map(|index| {
                assets.image(
                    section
                        .first(&format!("Frame{index}"))
                        .ok_or("Missing minimap frame")?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let composite = stitch(&frames);
        let size = [composite.width() as f32, composite.height() as f32];
        let mut crop = composite.clone();
        let mut fit = composite;
        for pixel in crop.pixels_mut() {
            pixel[3] = (u16::from(pixel[3]) * 170 / 255) as u8;
        }
        for pixel in fit.pixels_mut() {
            pixel[3] = (u16::from(pixel[3]) * 153 / 255) as u8;
        }
        let hero = catalog
            .section("hero")
            .and_then(|s| s.first("Frame0"))
            .ok_or("Missing minimap hero")?;
        let mut images = vec![
            ("@minimap-crop", crop),
            ("@minimap-fit", fit),
            ("@minimap-hero", assets.image(hero)?),
        ];
        let controls = SectionCatalog::parse(&assets.read("ani/Control.Ani")?);
        for (section, keys) in [
            ("Button00", ["@minimap-plus", "@minimap-plus-down"]),
            ("Button01", ["@minimap-minus", "@minimap-minus-down"]),
            ("Button02", ["@minimap-expand", "@minimap-expand-down"]),
        ] {
            let section = controls.section(section).ok_or("Missing minimap control")?;
            for (index, key) in keys.into_iter().enumerate() {
                images.push((
                    key,
                    assets.image(
                        section
                            .first(&format!("Frame{index}"))
                            .ok_or("Missing minimap control frame")?,
                    )?,
                ));
            }
        }
        Ok(Some(Self {
            images,
            size,
            single: count == 1,
            fitted: false,
            expanded: false,
            pressed: None,
        }))
    }

    pub fn pointer(&mut self, position: [f32; 2], down: bool, viewport: [f32; 2]) -> bool {
        let hit = [viewport[0] - 40.0, viewport[0] - 20.0]
            .into_iter()
            .position(|x| {
                position[0] >= x
                    && position[0] < x + 16.0
                    && position[1] >= 4.0
                    && position[1] < 20.0
            });
        if down {
            self.pressed = hit;
            return hit.is_some();
        }
        let pressed = self.pressed.take();
        if pressed.is_some() && pressed == hit {
            if hit == Some(0) {
                self.fitted = !self.fitted;
            } else {
                self.expanded = !self.expanded;
            }
        }
        pressed.is_some()
    }

    pub fn quads(
        &self,
        map: &Map,
        player: [f32; 2],
        camera: [f32; 2],
        viewport: [f32; 2],
    ) -> Vec<Quad> {
        let size = if self.expanded || self.single {
            self.size
        } else {
            [170.0, 128.0]
        };
        let rect = [viewport[0] - size[0], 0.0, size[0], size[1]];
        let fitted = self.fitted || self.expanded;
        let source = if fitted || self.single {
            [0.0, 0.0, self.size[0], self.size[1]]
        } else {
            let center =
                map.unproject([camera[0] + viewport[0] * 0.5, camera[1] + viewport[1] * 0.5]);
            crop(project(map, center, self.size), self.size, size)
        };
        let mut quads = vec![Quad {
            texture: if fitted && !self.single {
                "@minimap-fit"
            } else {
                "@minimap-crop"
            },
            rect,
            uv: [
                source[0] / self.size[0],
                source[1] / self.size[1],
                source[2] / self.size[0],
                source[3] / self.size[1],
            ],
        }];
        let point = project(map, player, self.size);
        let hero = &self.images[2].1;
        let marker = [
            rect[0] + (point[0] - source[0]) * rect[2] / source[2] - hero.width() as f32 * 0.5,
            rect[1] + (point[1] - source[1]) * rect[3] / source[3] - hero.height() as f32 * 0.5,
            hero.width() as f32,
            hero.height() as f32,
        ];
        if let Some(quad) = clipped("@minimap-hero", marker, rect) {
            quads.push(quad);
        }
        for (index, key) in [
            if self.fitted {
                ["@minimap-minus", "@minimap-minus-down"]
            } else {
                ["@minimap-plus", "@minimap-plus-down"]
            },
            ["@minimap-expand", "@minimap-expand-down"],
        ]
        .into_iter()
        .enumerate()
        {
            let key = key[usize::from(self.pressed == Some(index))];
            let image = &self.images.iter().find(|(name, _)| *name == key).unwrap().1;
            quads.push(Quad {
                texture: key,
                rect: [
                    viewport[0] - 40.0 + index as f32 * 20.0,
                    4.0,
                    image.width() as f32,
                    image.height() as f32,
                ],
                uv: [0.0, 0.0, 1.0, 1.0],
            });
        }
        quads
    }
}

fn stitch(frames: &[image::RgbaImage]) -> image::RgbaImage {
    if frames.len() == 1 {
        return frames[0].clone();
    }
    let left = frames[0].width().max(frames[2].width());
    let top = frames[0].height().max(frames[1].height());
    let mut image = image::RgbaImage::new(
        left + frames[1].width().max(frames[3].width()),
        top + frames[2].height().max(frames[3].height()),
    );
    for (frame, [x, y]) in frames
        .iter()
        .zip([[0, 0], [left, 0], [0, top], [left, top]])
    {
        image::imageops::replace(&mut image, frame, x.into(), y.into());
    }
    image
}

fn project(map: &Map, cell: [f32; 2], raster: [f32; 2]) -> [f32; 2] {
    project_cell(cell, [map.data.width, map.data.height], map.extent, raster)
}

fn project_cell(cell: [f32; 2], grid: [u32; 2], drawable: [f32; 2], raster: [f32; 2]) -> [f32; 2] {
    let world = [
        (cell[0] - cell[1]) * 32.0 + grid[0] as f32 * 32.0,
        (cell[0] + cell[1]) * 16.0 + 16.0,
    ];
    let origin = [
        grid[0] as f32 * 32.0 - drawable[0] * 0.5,
        grid[1] as f32 * 16.0 - drawable[1] * 0.5 + 16.0
            - if grid[1].is_multiple_of(2) { 16.0 } else { 0.0 },
    ];
    [
        (world[0] - origin[0]) / drawable[0] * raster[0],
        (world[1] - origin[1]) / drawable[1] * raster[1],
    ]
}

fn crop(center: [f32; 2], raster: [f32; 2], destination: [f32; 2]) -> [f32; 4] {
    let width = destination[0].min(raster[0]);
    let height = destination[1].min(raster[1]);
    [
        (center[0] - width * 0.5).clamp(0.0, raster[0] - width),
        (center[1] - height * 0.5).clamp(0.0, raster[1] - height),
        width,
        height,
    ]
}

fn clipped(texture: &'static str, rect: [f32; 4], bounds: [f32; 4]) -> Option<Quad> {
    let left = rect[0].max(bounds[0]);
    let top = rect[1].max(bounds[1]);
    let right = (rect[0] + rect[2]).min(bounds[0] + bounds[2]);
    let bottom = (rect[1] + rect[3]).min(bounds[1] + bounds[3]);
    if right <= left || bottom <= top {
        return None;
    }
    Some(Quad {
        texture,
        rect: [left, top, right - left, bottom - top],
        uv: [
            (left - rect[0]) / rect[2],
            (top - rect[1]) / rect[3],
            (right - left) / rect[2],
            (bottom - top) / rect[3],
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twin_city_projection_matches_client() {
        let point = project_cell(
            [425.0, 367.0],
            [972, 972],
            [26368.0, 17920.0],
            [753.0, 512.0],
        );
        assert!((point[0] - 429.5024).abs() < 0.001);
        assert!((point[1] - 174.1714).abs() < 0.001);
    }
    #[test]
    fn frames_are_row_major_with_unequal_widths() {
        let frames = [377, 376, 377, 376]
            .into_iter()
            .enumerate()
            .map(|(i, w)| image::RgbaImage::from_pixel(w, 256, image::Rgba([i as u8, 0, 0, 255])))
            .collect::<Vec<_>>();
        let image = stitch(&frames);
        assert_eq!(image.dimensions(), (753, 512));
        for (i, [x, y]) in [[0, 0], [377, 0], [0, 256], [377, 256]]
            .into_iter()
            .enumerate()
        {
            assert_eq!(image.get_pixel(x, y)[0], i as u8);
        }
    }
    #[test]
    fn crop_clamps_but_does_not_pin_markers() {
        assert_eq!(
            crop([0.0, 0.0], [753.0, 512.0], [170.0, 128.0]),
            [0.0, 0.0, 170.0, 128.0]
        );
        assert!(clipped("hero", [-30.0, 0.0, 16.0, 16.0], [0.0, 0.0, 170.0, 128.0]).is_none());
        assert_eq!(
            clipped("hero", [-8.0, 0.0, 16.0, 16.0], [0.0, 0.0, 170.0, 128.0])
                .unwrap()
                .uv,
            [0.5, 0.0, 0.5, 1.0]
        );
    }
}
