// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::action::Action;
use crate::assets::{Assets, Result};
use sandbox_data::{
    c3::{C3Document, C3Mesh, C3Motion, Matrix4, Vector3},
    ini::{IdPathCatalog, SectionCatalog},
};

pub struct Part {
    pub mesh: C3Mesh,
    pub texture: String,
    local: Option<C3Motion>,
    track: usize,
}
pub struct Character {
    pub parts: Vec<Part>,
    actions: Vec<C3Document>,
    intervals: [f32; 4],
    pub name: String,
    pub guild: String,
    pub guild_rank: String,
    pub health: u32,
    pub maximum_health: u32,
}
impl Character {
    pub fn load(assets: &mut Assets, body: u32) -> Result<Self> {
        if !(1..=4).contains(&body) {
            return Err("--body must be 1, 2, 3 or 4".into());
        }
        let profile = SectionCatalog::parse(include_bytes!("../../../character.ini"));
        let profile = profile
            .section("Character")
            .ok_or("Character profile missing")?;
        let number = |key: &str| -> Result<u32> {
            Ok(profile
                .first(key)
                .ok_or_else(|| format!("Missing profile {key}"))?
                .parse()?)
        };
        let armor = SectionCatalog::parse(&assets.read("ini/armor.ini")?);
        let entry = armor
            .section(&(body * 1_000_000 + number("Armor")? / 10 * 10).to_string())
            .ok_or("Missing armor in armor.ini")?;
        let models = IdPathCatalog::parse(&assets.read("ini/3dobj.ini")?);
        let textures = IdPathCatalog::parse(&assets.read("ini/3dtexture.ini")?);
        let motions = IdPathCatalog::parse(&assets.read("ini/3dmotion.ini")?);
        let model_id = entry.first("Mesh0").ok_or("Missing Mesh0")?.parse()?;
        let texture_id = entry.first("Texture0").ok_or("Missing Texture0")?.parse()?;
        let doc = C3Document::parse(
            &assets.read(models.get(model_id).ok_or("Body model missing")?.as_str())?,
        )?;
        let index = doc
            .meshes
            .iter()
            .position(|m| m.name == "v_body")
            .ok_or("Body mesh missing")?;
        let mesh = doc.meshes[index].clone();
        let right = number("RightWeapon")?;
        let left = number("LeftWeapon")?;
        let weapon_action = if right != 0 && left != 0 {
            600 + right % 100_000 / 10_000 * 10 + left % 100_000 / 10_000
        } else {
            right / 1000
        };
        let mut motion = |action| -> Result<C3Document> {
            let path = [
                body * 1_000_000 + weapon_action * 1000 + action,
                1_000_000 + weapon_action * 1000 + action,
                body * 1_000_000 + action,
                1_000_000 + action,
            ]
            .into_iter()
            .find_map(|key| motions.get(key))
            .ok_or("Body action missing")?;
            Ok(C3Document::parse(&assets.read(path.as_str())?)?)
        };
        let actions = Action::ALL
            .into_iter()
            .map(Action::catalog_id)
            .map(&mut motion)
            .collect::<Result<Vec<_>>>()?;
        let mut parts = vec![Part {
            mesh,
            texture: textures
                .get(texture_id)
                .ok_or("Body texture missing")?
                .as_str()
                .to_owned(),
            local: None,
            track: index,
        }];
        for (catalog, key, attachment) in [
            (
                "ini/armet.ini",
                body * 1_000_000 + 119_000 + number("Hair")?,
                "v_armet",
            ),
            ("ini/weapon.ini", right, "v_r_weapon"),
            ("ini/weapon.ini", left, "v_l_weapon"),
        ] {
            let definition = SectionCatalog::parse(&assets.read(catalog)?);
            let entry = definition
                .section(&key.to_string())
                .ok_or_else(|| format!("Missing part {key} in {catalog}"))?;
            let model = entry.first("Mesh0").ok_or("Part Mesh0 missing")?.parse()?;
            let texture = entry
                .first("Texture0")
                .ok_or("Part Texture0 missing")?
                .parse()?;
            let part = C3Document::parse(
                &assets.read(models.get(model).ok_or("Part model missing")?.as_str())?,
            )?;
            let track = doc
                .meshes
                .iter()
                .position(|m| m.name == attachment)
                .ok_or("Attachment missing")?;
            parts.push(Part {
                mesh: part.meshes.first().ok_or("Empty part")?.clone(),
                texture: textures
                    .get(texture)
                    .ok_or("Part texture missing")?
                    .as_str()
                    .to_owned(),
                local: Some(part.motions.first().ok_or("Part motion missing")?.clone()),
                track,
            });
        }
        let timing = assets.read("ini/Action.dat")?;
        let read = |offset: usize| -> Option<u32> {
            Some(u32::from_le_bytes(
                timing.get(offset..offset + 4)?.try_into().ok()?,
            ))
        };
        let count = read(0).ok_or("Action.dat header missing")? as usize;
        if count > 1_000_000 || timing.len() < 4 + count * 20 {
            return Err("Action.dat truncated".into());
        }
        let intervals = Action::ALL.map(|action| {
            let action = action.catalog_id();
            [
                body * 1_000_000 + weapon_action * 1000 + action,
                999_000_000 + weapon_action * 1000 + action,
                body * 1_000_000 + 999_000 + action,
                999_999_000 + action,
            ]
            .into_iter()
            .find_map(|key| {
                (0..count)
                    .find(|i| read(4 + i * 20 + 4) == Some(key))
                    .and_then(|i| read(4 + i * 20 + 12))
            })
            .unwrap_or(33)
            .max(5) as f32
        });
        for action in &actions {
            for part in &parts {
                if part.track >= action.motions.len() {
                    return Err("Missing attachment motion track".into());
                }
            }
        }
        Ok(Self {
            parts,
            actions,
            intervals,
            name: profile.first("Name").unwrap_or("dek").to_owned(),
            guild: profile.first("Guild").unwrap_or("").to_owned(),
            guild_rank: profile.first("GuildRank").unwrap_or("").to_owned(),
            health: number("Health")?,
            maximum_health: number("MaximumHealth")?,
        })
    }
    pub fn duration(&self, action: Action) -> std::time::Duration {
        let action = action.index();
        std::time::Duration::from_secs_f32(
            self.actions[action].motions[self.parts[0].track]
                .frame_count
                .max(1) as f32
                * self.intervals[action]
                / 1000.0,
        )
    }
    pub fn vertices(
        &self,
        part: &Part,
        seconds: f32,
        action: Action,
        facing: f32,
        progress: Option<f32>,
    ) -> Vec<([f32; 3], [f32; 2])> {
        let action = action.index();
        let track = &self.actions[action].motions[part.track];
        let frame = progress.map_or_else(
            || (seconds * 1000.0 / self.intervals[action]) % track.frame_count.max(1) as f32,
            |p| {
                (p.clamp(0.0, 1.0) * track.frame_count.max(1) as f32)
                    .min(track.frame_count.max(1) as f32 - f32::EPSILON)
            },
        );
        let motion = part.local.as_ref().unwrap_or(track);
        let attachment = if part.local.is_some() {
            track.matrix_at(0, frame)
        } else {
            Matrix4::IDENTITY
        };
        let palette: Vec<Matrix4> = (0..motion.bone_count)
            .map(|b| {
                part.mesh
                    .initial_matrix
                    .multiply(motion.matrix_at(b, frame))
                    .multiply(attachment)
            })
            .collect();
        let (s, c) = facing.sin_cos();
        let (ts, tc) = (-std::f32::consts::FRAC_PI_4).sin_cos();
        part.mesh
            .vertices
            .iter()
            .map(|v| {
                let transform = (0..2)
                    .find(|i| v.bone_weights[*i] > 0.0)
                    .and_then(|i| palette.get(v.bone_indices[i] as usize))
                    .unwrap_or(&part.mesh.initial_matrix);
                let p = transform.transform_point(v.position);
                let rotated = Vector3 {
                    x: p.x * c - p.y * s,
                    y: p.x * s + p.y * c,
                    z: p.z,
                };
                (
                    [
                        rotated.x * 0.75,
                        (rotated.y * ts + rotated.z * tc) * 0.75,
                        (rotated.y * tc - rotated.z * ts) * 0.75,
                    ],
                    [v.texture_coordinate.x, v.texture_coordinate.y],
                )
            })
            .collect()
    }
}
