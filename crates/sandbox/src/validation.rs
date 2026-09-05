// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::action::Action;
use crate::{assets::Result, character::Character, map::Map, movement::Player};

pub fn movement(map: &Map, character: &Character, spawn: [i32; 2]) -> Result<()> {
    let world = map.project(spawn.map(|v| v as f32));
    let roundtrip = map.unproject(world);
    if roundtrip.map(|v| v.round() as i32) != spawn {
        return Err("Projection roundtrip failed".into());
    }
    let target = (-6..=6)
        .flat_map(|x| (-6..=6).map(move |y| [spawn[0] + x, spawn[1] + y]))
        .find(|p| *p != spawn && map.walkable(*p))
        .ok_or("No nearby movement test cell")?;
    for running in [false, true] {
        let mut player = Player::new(map, Some(spawn))?;
        player.running = running;
        player.go(map, target);
        if !player.moving() {
            return Err("Asset-backed route was not created".into());
        }
        let mut last_instance = 0;
        let mut steps = Vec::new();
        for _ in 0..3600 {
            player.tick(1.0 / 120.0);
            if player.action_instance != last_instance {
                let expected = match (running, steps.len() % 2 == 1) {
                    (false, false) => Action::WalkLeft,
                    (false, true) => Action::WalkRight,
                    (true, false) => Action::RunLeft,
                    (true, true) => Action::RunRight,
                };
                if player.action != expected {
                    return Err("Movement did not alternate left/right actions".into());
                }
                last_instance = player.action_instance;
                steps.push(player.action);
            }
            if !player.moving() {
                break;
            }
        }
        if player.cell() != target {
            return Err("Route did not reach destination".into());
        }
        if steps.len() < 2 {
            return Err("Movement check needs both foot actions".into());
        }
    }
    for action in Action::ALL {
        for part in &character.parts {
            for progress in [0.0, 0.5, 1.0] {
                let vertices = character.vertices(part, 0.0, action, 0.0, Some(progress));
                if vertices.is_empty()
                    || vertices
                        .iter()
                        .any(|(p, uv)| p.iter().chain(uv).any(|v| !v.is_finite()))
                {
                    return Err("Movement pose contains invalid geometry".into());
                }
            }
        }
    }
    let mut player = Player::new(map, Some(spawn))?;
    player.jump(map, target, character.duration(Action::Jump));
    if !player.moving() {
        return Err("Asset-backed jump rejected".into());
    }
    let mut peak = 0.0_f32;
    for _ in 0..1200 {
        player.tick(1.0 / 120.0);
        peak = peak.max(player.height);
        if !player.moving() {
            break;
        }
    }
    if peak <= 0.0 || player.height != 0.0 || !map.walkable(player.cell()) || player.cell() == spawn
    {
        return Err("Jump/landing validation failed".into());
    }
    println!("Movement OK: walk, run, jump arc/landing, projection; jump peak {peak:.1}px");
    Ok(())
}
