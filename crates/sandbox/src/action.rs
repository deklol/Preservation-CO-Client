// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    Idle,
    WalkLeft,
    WalkRight,
    RunLeft,
    RunRight,
    Jump,
}

impl Action {
    pub const ALL: [Self; 6] = [
        Self::Idle,
        Self::WalkLeft,
        Self::WalkRight,
        Self::RunLeft,
        Self::RunRight,
        Self::Jump,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::WalkLeft => 1,
            Self::WalkRight => 2,
            Self::RunLeft => 3,
            Self::RunRight => 4,
            Self::Jump => 5,
        }
    }

    pub const fn catalog_id(self) -> u32 {
        match self {
            Self::Idle => 100,
            Self::WalkLeft => 110,
            Self::WalkRight => 111,
            Self::RunLeft => 120,
            Self::RunRight => 121,
            Self::Jump => 130,
        }
    }

    pub const fn paired_action(self) -> Option<Self> {
        match self {
            Self::WalkRight => Some(Self::WalkLeft),
            Self::RunRight => Some(Self::RunLeft),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;

    #[test]
    fn actions_keep_their_catalog_and_storage_order() {
        assert_eq!(Action::ALL.map(Action::index), [0, 1, 2, 3, 4, 5]);
        assert_eq!(
            Action::ALL.map(Action::catalog_id),
            [100, 110, 111, 120, 121, 130]
        );
    }
}
