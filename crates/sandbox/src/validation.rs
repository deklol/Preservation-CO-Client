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
        for _ in 0..3600 {
            player.tick(1.0 / 120.0);
            if !player.moving() {
                break;
            }
        }
        if player.cell() != target {
            return Err("Route did not reach destination".into());
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
