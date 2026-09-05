// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::{ContentError, SliceReader, VirtualPath, tq_archive_hash};
use std::collections::HashMap;

const HEADER_SIGNATURE: u32 = 0x5744_4650;
const DIRECTORY_ENTRY_LENGTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WdfHeader {
    pub entry_count: u32,
    pub directory_offset: u32,
}

impl WdfHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("WDF header", bytes);
        let signature = reader.read_u32_le()?;
        if signature != HEADER_SIGNATURE {
            return Err(ContentError::InvalidData {
                context: "WDF header",
                detail: format!("unexpected signature 0x{signature:08X}"),
            });
        }

        Ok(Self {
            entry_count: reader.read_u32_le()?,
            directory_offset: reader.read_u32_le()?,
        })
    }

    pub fn directory_length(self) -> Result<usize, ContentError> {
        usize::try_from(self.entry_count)
            .ok()
            .and_then(|count| count.checked_mul(DIRECTORY_ENTRY_LENGTH))
            .ok_or(ContentError::InvalidData {
                context: "WDF header",
                detail: "directory length overflows this platform".to_owned(),
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WdfEntry {
    pub id: u32,
    pub offset: u32,
    pub size: u32,
    pub reserved: u32,
}

#[derive(Clone, Debug, Default)]
pub struct WdfDirectory {
    entries: HashMap<u32, WdfEntry>,
}

impl WdfDirectory {
    pub fn parse(
        header: WdfHeader,
        directory_bytes: &[u8],
        archive_length: u64,
    ) -> Result<Self, ContentError> {
        let expected_length = header.directory_length()?;
        if directory_bytes.len() != expected_length {
            return Err(ContentError::InvalidData {
                context: "WDF directory",
                detail: format!(
                    "expected {expected_length} bytes for {} entries, received {}",
                    header.entry_count,
                    directory_bytes.len()
                ),
            });
        }

        let mut reader = SliceReader::new("WDF directory", directory_bytes);
        let mut entries = HashMap::with_capacity(header.entry_count as usize);
        for _ in 0..header.entry_count {
            let entry = WdfEntry {
                id: reader.read_u32_le()?,
                offset: reader.read_u32_le()?,
                size: reader.read_u32_le()?,
                reserved: reader.read_u32_le()?,
            };
            let end = u64::from(entry.offset) + u64::from(entry.size);
            if end > archive_length {
                return Err(ContentError::InvalidData {
                    context: "WDF directory",
                    detail: format!("entry {} ends at {end}, beyond {archive_length}", entry.id),
                });
            }
            if entries.insert(entry.id, entry).is_some() {
                return Err(ContentError::InvalidData {
                    context: "WDF directory",
                    detail: format!("entry {} appears more than once", entry.id),
                });
            }
        }

        Ok(Self { entries })
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<WdfEntry> {
        self.entries.get(&id).copied()
    }

    #[must_use]
    pub fn contains_id(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn path_id(path: &VirtualPath) -> Result<u32, ContentError> {
    let normalized = path.as_str().to_ascii_lowercase();
    tq_archive_hash::path_id(&normalized, "WDF virtual path")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monster_model_id_matches_the_target_archive() {
        let path = VirtualPath::try_from("c3\\monster\\104N\\104000000.c3").unwrap();

        assert_eq!(path_id(&path).unwrap(), 0xB8D9_C538);
    }

    #[test]
    fn case_and_separator_variants_address_the_same_entry() {
        let forward = VirtualPath::try_from("c3/monster/104n/104000000.c3").unwrap();
        let windows = VirtualPath::try_from("C3\\MONSTER\\104N\\104000000.C3").unwrap();
        assert_eq!(path_id(&forward), path_id(&windows));
    }

    #[test]
    fn directory_parser_rejects_entries_outside_the_archive() {
        let header_bytes = [0x50, 0x46, 0x44, 0x57, 1, 0, 0, 0, 12, 0, 0, 0];
        let header = WdfHeader::parse(&header_bytes).unwrap();
        let entry_bytes = [1, 0, 0, 0, 90, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0];

        assert!(WdfDirectory::parse(header, &entry_bytes, 100).is_err());
    }
}
