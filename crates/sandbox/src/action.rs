// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    Idle,
    Walk,
    Run,
    Jump,
}

impl Action {
    pub const ALL: [Self; 4] = [Self::Idle, Self::Walk, Self::Run, Self::Jump];

    pub const fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Walk => 1,
            Self::Run => 2,
            Self::Jump => 3,
        }
    }

    pub const fn catalog_id(self) -> u32 {
        match self {
            Self::Idle => 100,
            Self::Walk => 110,
            Self::Run => 120,
            Self::Jump => 130,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;

    #[test]
    fn actions_keep_their_catalog_and_storage_order() {
        assert_eq!(Action::ALL.map(Action::index), [0, 1, 2, 3]);
        assert_eq!(Action::ALL.map(Action::catalog_id), [100, 110, 120, 130]);
    }
}
