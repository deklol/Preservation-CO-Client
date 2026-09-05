// Source: @digitalm1nd on x.com / _dek on Discord - Preservation Conquer project - https://discord.gg/CvKPXEHYRY
#[must_use]
pub const fn resolve_weapon_action(right: Option<u32>, left: Option<u32>) -> u32 {
    match (
        match right {
            Some(value) if value != 0 => Some(value),
            _ => None,
        },
        match left {
            Some(value) if value != 0 => Some(value),
            _ => None,
        },
    ) {
        (Some(right), None) => right / 1_000,
        (None, Some(_)) => 741,
        (Some(right), Some(left)) if left / 100_000 == 9 => 700 + right / 10_000,
        (Some(right), Some(left)) if left / 100_000 == 4 => {
            600 + right % 100_000 / 10_000 * 10 + left % 100_000 / 10_000
        }
        (Some(right), Some(_)) if right / 1_000 == 500 => 500,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_weapon_action;

    #[test]
    fn exact_dual_weapon_families_are_stable() {
        assert_eq!(resolve_weapon_action(Some(410_339), None), 410);
        assert_eq!(resolve_weapon_action(Some(410_339), Some(900_000)), 741);
        assert_eq!(resolve_weapon_action(Some(410_339), Some(420_339)), 612);
        assert_eq!(resolve_weapon_action(Some(500_329), None), 500);
    }
}
