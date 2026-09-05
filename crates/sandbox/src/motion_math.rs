// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use std::f32::consts::{FRAC_PI_4, PI};
use std::time::Duration;

pub const WALK_CELLS_PER_SECOND: f32 = 3.0;
pub const RUN_CELLS_PER_SECOND: f32 = 6.0;
pub const MAX_JUMP_DISTANCE: f32 = 16.0;
pub const MAX_JUMP_CLIMB: i32 = 120;
pub const MAX_JUMP_RISE: i32 = 210;

const WORLD_PIXELS_PER_UNIT: f32 = 32.0;
const JUMP_ARC_SCALE: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpMotion {
    pub duration: Duration,
    pub peak_screen_pixels: f32,
}

#[must_use]
pub fn step_duration(running: bool) -> Duration {
    let cells_per_second = if running {
        RUN_CELLS_PER_SECOND
    } else {
        WALK_CELLS_PER_SECOND
    };
    Duration::from_secs_f32(1.0 / cells_per_second)
}

#[must_use]
pub fn jump_motion(start: [f32; 2], target: [f32; 2], animation_duration: Duration) -> JumpMotion {
    let distance = world_distance(start, target);
    let maximum_world_distance = world_distance([0.0, 0.0], [MAX_JUMP_DISTANCE, 0.0]);
    let distance_fraction = (distance / maximum_world_distance).max(0.4);
    let duration = (animation_duration.as_secs_f32() * 2.0 * distance_fraction).max(0.5);
    let peak_world_units = (distance * JUMP_ARC_SCALE).max(0.5);

    JumpMotion {
        duration: Duration::from_secs_f32(duration),
        peak_screen_pixels: peak_world_units * FRAC_PI_4.sin() * WORLD_PIXELS_PER_UNIT,
    }
}

#[must_use]
pub fn jump_screen_height(peak_screen_pixels: f32, progress: f32) -> f32 {
    peak_screen_pixels * (PI * progress.clamp(0.0, 1.0)).sin()
}

#[must_use]
pub fn cell_distance(start: [f32; 2], target: [f32; 2]) -> f32 {
    (target[0] - start[0]).hypot(target[1] - start[1])
}

#[must_use]
pub fn clamp_jump_target(start: [f32; 2], target: [f32; 2]) -> [f32; 2] {
    let distance = cell_distance(start, target);
    if distance <= MAX_JUMP_DISTANCE || distance <= f32::EPSILON {
        return target;
    }
    let scale = MAX_JUMP_DISTANCE / distance;
    [
        (target[0] - start[0]).mul_add(scale, start[0]),
        (target[1] - start[1]).mul_add(scale, start[1]),
    ]
}

#[must_use]
pub fn direction_towards(start: [f32; 2], target: [f32; 2]) -> u8 {
    let delta_x = target[0] - start[0];
    let delta_y = target[1] - start[1];

    if delta_x.abs() <= f32::EPSILON && delta_y.abs() <= f32::EPSILON {
        return 3;
    }

    if delta_x.abs() <= f32::EPSILON {
        return if delta_y > 0.0 { 0 } else { 4 };
    }
    if delta_y.abs() <= f32::EPSILON {
        return if delta_x > 0.0 { 6 } else { 2 };
    }

    let signed_y = delta_y * 100.0 * delta_x.signum();
    let abs_x = delta_x.abs();
    let bands = [-241.0 * abs_x, -41.0 * abs_x, 41.0 * abs_x, 241.0 * abs_x];
    let sector = if signed_y >= bands[0] && signed_y < bands[1] {
        0
    } else if signed_y >= bands[1] && signed_y < bands[2] {
        1
    } else if signed_y >= bands[2] && signed_y < bands[3] {
        2
    } else {
        3
    };
    match sector {
        0 => {
            if delta_x > 0.0 {
                5
            } else {
                1
            }
        }
        1 => {
            if delta_x > 0.0 {
                6
            } else {
                2
            }
        }
        2 => {
            if delta_x > 0.0 {
                7
            } else {
                3
            }
        }
        _ => {
            if delta_y > 0.0 {
                0
            } else {
                4
            }
        }
    }
}

#[must_use]
pub fn jump_facing(start: [f32; 2], target: [f32; 2]) -> Option<f32> {
    let delta_x = target[0] - start[0];
    let delta_y = target[1] - start[1];
    if delta_x.abs() <= f32::EPSILON && delta_y.abs() <= f32::EPSILON {
        return None;
    }

    let world_x = delta_x - delta_y;
    let world_depth = (delta_x + delta_y) * 0.5;
    Some(world_x.atan2(world_depth))
}

fn world_distance(start: [f32; 2], target: [f32; 2]) -> f32 {
    let start = cell_to_world(start);
    let target = cell_to_world(target);
    (target[0] - start[0]).hypot(target[1] - start[1])
}

fn cell_to_world(cell: [f32; 2]) -> [f32; 2] {
    [cell[0] - cell[1], (cell[0] + cell[1]) * 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_rates_match_the_documented_timing() {
        assert_eq!(step_duration(false), Duration::from_secs_f32(1.0 / 3.0));
        assert_eq!(step_duration(true), Duration::from_secs_f32(1.0 / 6.0));
    }

    #[test]
    fn direction_math_matches_the_eight_way_wire_order() {
        assert_eq!(direction_towards([10.0, 10.0], [10.0, 11.0]), 0);
        assert_eq!(direction_towards([10.0, 10.0], [9.0, 9.0]), 3);
        assert_eq!(direction_towards([10.0, 10.0], [11.0, 10.0]), 6);
        assert_eq!(direction_towards([10.0, 10.0], [20.0, 11.0]), 6);
        assert_eq!(direction_towards([10.0, 10.0], [10.0, 10.0]), 3);
    }

    #[test]
    fn arbitrary_clicks_quantize_in_visible_isometric_space() {
        let start = [100.0, 100.0];
        let cases = [
            ([92.0, 92.0], 3),
            ([100.0, 92.0], 4),
            ([108.0, 92.0], 5),
            ([108.0, 100.0], 6),
            ([108.0, 108.0], 7),
            ([100.0, 108.0], 0),
            ([92.0, 108.0], 1),
            ([92.0, 100.0], 2),
        ];
        for (target, expected) in cases {
            assert_eq!(direction_towards(start, target), expected);
        }
    }

    #[test]
    fn direction_math_preserves_redux_tangent_boundaries() {
        let start = [100.0, 100.0];
        assert_eq!(direction_towards(start, [117.0, 93.0]), 5);
        assert_eq!(direction_towards(start, [117.0, 107.0]), 7);
        assert_eq!(direction_towards(start, [83.0, 93.0]), 3);
        assert_eq!(direction_towards(start, [83.0, 107.0]), 1);
    }

    #[test]
    fn jump_facing_preserves_angles_between_wire_sectors() {
        let north = jump_facing([0.0, 0.0], [-1.0, -1.0]).unwrap();
        let between_north_and_east = jump_facing([0.0, 0.0], [1.0, -3.0]).unwrap();
        let east = jump_facing([0.0, 0.0], [1.0, -1.0]).unwrap();

        assert!((north.abs() - PI).abs() < 0.001);
        assert!(between_north_and_east > east);
        assert!(between_north_and_east < north);
        assert!(jump_facing([4.0, 4.0], [4.0, 4.0]).is_none());
    }

    #[test]
    fn jump_target_is_clamped_to_sixteen_cells() {
        let target = clamp_jump_target([10.0, 10.0], [42.0, 10.0]);
        assert_eq!(target, [26.0, 10.0]);
    }
}
