// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VirtualPath(String);

impl VirtualPath {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for VirtualPath {
    type Error = ContentError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().replace('\\', "/");
        if normalized.is_empty()
            || normalized.starts_with('/')
            || normalized.contains(':')
            || normalized
                .split('/')
                .any(|part| part.is_empty() || part == "..")
        {
            return Err(ContentError::InvalidPath(value.to_owned()));
        }

        Ok(Self(normalized))
    }
}

impl Display for VirtualPath {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentError {
    InvalidPath(String),
    NotFound(VirtualPath),
    Truncated {
        context: &'static str,
        offset: usize,
        needed: usize,
        available: usize,
    },
    InvalidData {
        context: &'static str,
        detail: String,
    },
}

impl Display for ContentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(formatter, "invalid virtual path: {path}"),
            Self::NotFound(path) => write!(formatter, "content not found: {path}"),
            Self::Truncated {
                context,
                offset,
                needed,
                available,
            } => write!(
                formatter,
                "truncated {context} at {offset}: need {needed} bytes, have {available}"
            ),
            Self::InvalidData { context, detail } => {
                write!(formatter, "invalid {context}: {detail}")
            }
        }
    }
}

impl Error for ContentError {}

pub trait ContentSource {
    fn read(&self, path: &VirtualPath) -> Result<Vec<u8>, ContentError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_paths_are_platform_independent() {
        let path = VirtualPath::try_from("data\\Map\\map\\map1002.DMap").unwrap();
        assert_eq!(path.as_str(), "data/Map/map/map1002.DMap");
    }

    #[test]
    fn virtual_paths_cannot_escape_the_content_root() {
        assert!(VirtualPath::try_from("../Server.dat").is_err());
        assert!(VirtualPath::try_from("data//map.dmap").is_err());
        assert!(VirtualPath::try_from("/data/map.dmap").is_err());
    }
}
