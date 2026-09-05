// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::action::Action;
use crate::map::Map;
use crate::motion_math::*;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, VecDeque},
};

struct Segment {
    start: [f32; 2],
    target: [f32; 2],
    elapsed: f32,
    duration: f32,
    peak: f32,
    jump: bool,
}
pub struct Player {
    pub position: [f32; 2],
    pub facing: f32,
    pub running: bool,
    pub height: f32,
    pub action: Action,
    pub action_instance: u64,
    right_foot: bool,
    route: VecDeque<[i32; 2]>,
    segment: Option<Segment>,
}
impl Player {
    pub fn new(map: &Map, spawn: Option<[i32; 2]>) -> Result<Self, String> {
        let spawn = spawn
            .or_else(|| {
                let center = [map.data.width as i32 / 2, map.data.height as i32 / 2];
                (0..map.data.width.max(map.data.height) as i32).find_map(|r| {
                    (-r..=r)
                        .flat_map(|i| {
                            [
                                [center[0] + i, center[1] - r],
                                [center[0] + i, center[1] + r],
                                [center[0] - r, center[1] + i],
                                [center[0] + r, center[1] + i],
                            ]
                        })
                        .find(|p| map.walkable(*p))
                })
            })
            .ok_or("Map has no walkable spawn")?;
        if !map.walkable(spawn) {
            return Err("Requested spawn is blocked or outside map".into());
        }
        Ok(Self {
            position: spawn.map(|v| v as f32),
            facing: -std::f32::consts::FRAC_PI_4,
            running: true,
            height: 0.0,
            action: Action::Idle,
            action_instance: 0,
            right_foot: false,
            route: VecDeque::new(),
            segment: None,
        })
    }
    pub fn cell(&self) -> [i32; 2] {
        self.position.map(|v| v.round() as i32)
    }
    pub fn moving(&self) -> bool {
        self.segment.is_some() || !self.route.is_empty()
    }
    pub fn progress(&self) -> Option<f32> {
        self.segment
            .as_ref()
            .map(|s| (s.elapsed / s.duration).clamp(0.0, 1.0))
    }
    pub fn stop(&mut self) {
        self.route.clear();
    }
    pub fn go(&mut self, map: &Map, target: [i32; 2]) {
        let start = self
            .segment
            .as_ref()
            .map_or(self.cell(), |s| s.target.map(|v| v as i32));
        if let Some(route) = route(start, target, |p| map.walkable(p)) {
            self.route = route.into();
        }
    }
    pub fn jump(&mut self, map: &Map, requested: [i32; 2], animation: std::time::Duration) {
        if self.segment.is_some() {
            return;
        }
        let start = self.cell();
        let clamped =
            clamp_jump_target(self.position, requested.map(|v| v as f32)).map(|v| v.round() as i32);
        let Some(target) = line(start, clamped)
            .into_iter()
            .skip(1)
            .rfind(|p| jump_allowed(map, start, *p))
        else {
            return;
        };
        let target = target.map(|v| v as f32);
        let motion = jump_motion(self.position, target, animation);
        self.facing = jump_facing(self.position, target).unwrap_or(self.facing);
        self.route.clear();
        self.action = Action::Jump;
        self.action_instance = self.action_instance.wrapping_add(1);
        self.segment = Some(Segment {
            start: self.position,
            target,
            elapsed: 0.0,
            duration: motion.duration.as_secs_f32(),
            peak: motion.peak_screen_pixels,
            jump: true,
        });
    }
    pub fn face(&mut self, target: [i32; 2]) {
        if !self.moving() {
            self.facing =
                -f32::from(direction_towards(self.position, target.map(|v| v as f32)) + 1)
                    * std::f32::consts::FRAC_PI_4;
        }
    }
    pub fn tick(&mut self, dt: f32) {
        let mut remaining = dt.clamp(0.0, 0.1);
        loop {
            if self.segment.is_none() {
                if let Some(target) = self.route.pop_front() {
                    let target = target.map(|v| v as f32);
                    self.facing = -f32::from(direction_towards(self.position, target) + 1)
                        * std::f32::consts::FRAC_PI_4;
                    self.action = match (self.running, self.right_foot) {
                        (false, false) => Action::WalkLeft,
                        (false, true) => Action::WalkRight,
                        (true, false) => Action::RunLeft,
                        (true, true) => Action::RunRight,
                    };
                    self.right_foot = !self.right_foot;
                    self.action_instance = self.action_instance.wrapping_add(1);
                    self.segment = Some(Segment {
                        start: self.position,
                        target,
                        elapsed: 0.0,
                        duration: step_duration(self.running).as_secs_f32(),
                        peak: 0.0,
                        jump: false,
                    });
                } else {
                    self.action = Action::Idle;
                    return;
                }
            }
            let segment = self.segment.as_mut().unwrap();
            let used = remaining.min(segment.duration - segment.elapsed);
            segment.elapsed += used;
            remaining -= used;
            let progress = (segment.elapsed / segment.duration).min(1.0);
            self.position = [
                segment.start[0] + (segment.target[0] - segment.start[0]) * progress,
                segment.start[1] + (segment.target[1] - segment.start[1]) * progress,
            ];
            self.height = if segment.jump {
                jump_screen_height(segment.peak, progress)
            } else {
                0.0
            };
            if progress >= 1.0 {
                self.position = segment.target;
                self.height = 0.0;
                self.segment = None;
                self.action = Action::Idle;
            }
            if self.segment.is_some() {
                break;
            }
        }
    }
}
fn line(start: [i32; 2], end: [i32; 2]) -> Vec<[i32; 2]> {
    let steps = (end[0] - start[0])
        .abs()
        .max((end[1] - start[1]).abs())
        .max(1);
    (0..=steps)
        .map(|i| {
            [0, 1].map(|axis| {
                (start[axis] as f32 + (end[axis] - start[axis]) as f32 * i as f32 / steps as f32)
                    .round() as i32
            })
        })
        .collect()
}
fn jump_allowed(map: &Map, start: [i32; 2], target: [i32; 2]) -> bool {
    if start == target
        || !map.walkable(target)
        || cell_distance(start.map(|v| v as f32), target.map(|v| v as f32)) > MAX_JUMP_DISTANCE
    {
        return false;
    }
    let Some(a) = map.data.cell(start[0] as u32, start[1] as u32) else {
        return false;
    };
    let Some(b) = map.data.cell(target[0] as u32, target[1] as u32) else {
        return false;
    };
    if i32::from(b.elevation) - i32::from(a.elevation) > MAX_JUMP_CLIMB {
        return false;
    }
    let ceiling = i32::from(a.elevation.max(b.elevation)) + MAX_JUMP_RISE;
    line(start, target).into_iter().skip(1).all(|p| {
        map.data
            .cell(p[0] as u32, p[1] as u32)
            .is_some_and(|c| i32::from(c.elevation) <= ceiling)
    })
}
fn route(
    start: [i32; 2],
    goal: [i32; 2],
    open: impl Fn([i32; 2]) -> bool,
) -> Option<Vec<[i32; 2]>> {
    if !open(goal) {
        return None;
    }
    let heuristic = |p: [i32; 2]| (p[0] - goal[0]).abs().max((p[1] - goal[1]).abs()) * 10;
    let mut queue = BinaryHeap::from([Reverse((heuristic(start), 0, start))]);
    let mut costs = HashMap::from([(start, 0)]);
    let mut parents = HashMap::new();
    for _ in 0..100_000 {
        let Reverse((_, cost, p)) = queue.pop()?;
        if cost != costs[&p] {
            continue;
        }
        if p == goal {
            let mut path = Vec::new();
            let mut p = p;
            while p != start {
                path.push(p);
                p = parents[&p];
            }
            path.reverse();
            return Some(path);
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let q = [p[0] + dx, p[1] + dy];
                if !open(q)
                    || (dx != 0
                        && dy != 0
                        && (!open([p[0] + dx, p[1]]) || !open([p[0], p[1] + dy])))
                {
                    continue;
                }
                let next = cost + if dx != 0 && dy != 0 { 14 } else { 10 };
                if costs.get(&q).is_none_or(|old| next < *old) {
                    costs.insert(q, next);
                    parents.insert(q, p);
                    queue.push(Reverse((next + heuristic(q), next, q)));
                }
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_around_wall() {
        let open = |p: [i32; 2]| {
            p[0] >= 0 && p[1] >= 0 && p[0] < 6 && p[1] < 6 && !(p[0] == 2 && p[1] < 4)
        };
        let path = route([0, 0], [4, 0], open).unwrap();
        assert!(path.iter().all(|p| open(*p)));
        assert!(path.iter().any(|p| p[1] >= 4));
    }
    #[test]
    fn rejects_corner_cut_and_blocked_target() {
        assert!(route([0, 0], [1, 1], |p| p == [0, 0] || p == [1, 1]).is_none());
        assert!(route([0, 0], [2, 2], |p| p == [0, 0]).is_none());
    }
}
