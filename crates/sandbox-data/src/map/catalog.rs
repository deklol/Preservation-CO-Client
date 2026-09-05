// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use super::{MAXIMUM_MAP_RECORDS, invalid, virtual_path};
use crate::{ContentError, SliceReader, VirtualPath};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameMapRecord {
    pub id: u32,
    pub dmap_path: VirtualPath,
    pub tile_size: u32,
}

#[derive(Clone, Debug, Default)]
pub struct GameMapCatalog {
    records: HashMap<u32, GameMapRecord>,
}

impl GameMapCatalog {
    pub fn parse(bytes: &[u8]) -> Result<Self, ContentError> {
        let mut reader = SliceReader::new("GameMap.dat", bytes);
        let count = reader.read_u32_le()?;
        if count > MAXIMUM_MAP_RECORDS {
            return Err(invalid(format!("map record count {count} is unreasonable")));
        }

        let mut records = HashMap::with_capacity(count as usize);
        for _ in 0..count {
            let id = reader.read_u32_le()?;
            let path_length = usize::try_from(reader.read_u32_le()?)
                .map_err(|_| invalid("GameMap.dat path length does not fit this platform"))?;
            if path_length == 0 || path_length > 512 {
                return Err(invalid(format!(
                    "invalid GameMap.dat path length {path_length}"
                )));
            }
            let raw_path = reader.read_exact(path_length)?;
            let path_end = raw_path
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(raw_path.len());
            if !raw_path[..path_end].is_ascii() {
                return Err(invalid("GameMap.dat path contains non-ASCII bytes"));
            }
            let path = String::from_utf8_lossy(&raw_path[..path_end]).replace('\\', "/");
            let tile_size = reader.read_u32_le()?;
            if tile_size == 0 || tile_size > 4096 {
                return Err(invalid(format!(
                    "invalid tile size {tile_size} for map {id}"
                )));
            }
            let record = GameMapRecord {
                id,
                dmap_path: virtual_path(&path, "GameMap.dat")?,
                tile_size,
            };
            if records.insert(id, record).is_some() {
                return Err(invalid(format!("map {id} appears more than once")));
            }
        }
        if reader.remaining() != 0 {
            return Err(invalid(format!(
                "GameMap.dat has {} unexplained trailing bytes",
                reader.remaining()
            )));
        }
        Ok(Self { records })
    }

    pub fn get(&self, id: u32) -> Option<&GameMapRecord> {
        self.records.get(&id)
    }

    pub fn merge_distinct(&mut self, extension: Self) -> Result<(), ContentError> {
        for (id, record) in extension.records {
            if self.records.contains_key(&id) {
                return Err(invalid(format!(
                    "extension map {id} conflicts with an existing map record"
                )));
            }
            self.records.insert(id, record);
        }
        Ok(())
    }

    pub fn encode(records: &[GameMapRecord]) -> Result<Vec<u8>, ContentError> {
        if records.len() > MAXIMUM_MAP_RECORDS as usize {
            return Err(invalid("map record count is unreasonable"));
        }
        let mut sorted = records.to_vec();
        sorted.sort_unstable_by_key(|record| record.id);
        if sorted
            .windows(2)
            .any(|records| records[0].id == records[1].id)
        {
            return Err(invalid("map record ID appears more than once"));
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(sorted.len() as u32).to_le_bytes());
        for record in sorted {
            if record.tile_size == 0 || record.tile_size > 4096 {
                return Err(invalid(format!(
                    "invalid tile size {} for map {}",
                    record.tile_size, record.id
                )));
            }
            let path = record.dmap_path.as_str().as_bytes();
            if path.is_empty() || path.len() >= 512 {
                return Err(invalid(format!(
                    "invalid DMap path length {} for map {}",
                    path.len(),
                    record.id
                )));
            }
            let length = u32::try_from(path.len() + 1)
                .map_err(|_| invalid("DMap path length does not fit the catalogue"))?;
            bytes.extend_from_slice(&record.id.to_le_bytes());
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(path);
            bytes.push(0);
            bytes.extend_from_slice(&record.tile_size.to_le_bytes());
        }
        Ok(bytes)
    }

    pub fn records(&self) -> impl Iterator<Item = &GameMapRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_null_terminated_paths_without_hard_coding_map_ids() {
        let mut bytes = 1_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&1002_u32.to_le_bytes());
        bytes.extend_from_slice(&19_u32.to_le_bytes());
        bytes.extend_from_slice(b"map/map/plain.DMap\0");
        bytes.extend_from_slice(&256_u32.to_le_bytes());
        let catalog = GameMapCatalog::parse(&bytes).unwrap();
        assert_eq!(
            catalog.get(1002).unwrap().dmap_path.as_str(),
            "map/map/plain.DMap"
        );
        assert_eq!(
            catalog
                .records()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            [1002]
        );
    }

    #[test]
    fn extension_catalogues_round_trip_and_cannot_shadow_stock_ids() {
        let record = GameMapRecord {
            id: 9001,
            dmap_path: VirtualPath::try_from("map/map/champaign.DMap").unwrap(),
            tile_size: 128,
        };
        let extension = GameMapCatalog::parse(&GameMapCatalog::encode(&[record]).unwrap()).unwrap();
        assert_eq!(extension.get(9001).unwrap().tile_size, 128);

        let mut base = extension.clone();
        assert!(base.merge_distinct(extension).is_err());
    }
}
