// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use crate::VirtualPath;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogDiagnostic {
    pub line: usize,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct IdPathCatalog {
    entries: HashMap<u32, VirtualPath>,
    diagnostics: Vec<CatalogDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IniProperty {
    pub key: String,
    pub value: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IniSection {
    pub name: String,
    pub line: usize,
    properties: Vec<IniProperty>,
}

impl IniSection {
    #[must_use]
    pub fn properties(&self) -> &[IniProperty] {
        &self.properties
    }

    #[must_use]
    pub fn first(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.key.eq_ignore_ascii_case(key))
            .map(|property| property.value.as_str())
    }
}

#[derive(Clone, Debug, Default)]
pub struct SectionCatalog {
    sections: Vec<IniSection>,
    diagnostics: Vec<CatalogDiagnostic>,
}

impl SectionCatalog {
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
        let mut catalog = Self::default();
        let mut current_section = None;

        for (index, raw_line) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty()
                || line.starts_with(';')
                || line.starts_with('#')
                || line.starts_with("//")
            {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let name = line[1..line.len() - 1].trim();
                if name.is_empty() {
                    catalog.diagnostics.push(CatalogDiagnostic {
                        line: line_number,
                        detail: "empty section name".to_owned(),
                    });
                    current_section = None;
                    continue;
                }

                catalog.sections.push(IniSection {
                    name: name.to_owned(),
                    line: line_number,
                    properties: Vec::new(),
                });
                current_section = Some(catalog.sections.len() - 1);
                continue;
            }

            let Some((raw_key, raw_value)) = line.split_once('=') else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: "expected key=value".to_owned(),
                });
                continue;
            };
            let key = raw_key.trim();
            if key.is_empty() {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: "empty property name".to_owned(),
                });
                continue;
            }
            let Some(section_index) = current_section else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: format!("property {key} appears before a section"),
                });
                continue;
            };

            catalog.sections[section_index]
                .properties
                .push(IniProperty {
                    key: key.to_owned(),
                    value: raw_value.trim().to_owned(),
                    line: line_number,
                });
        }

        catalog
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    #[must_use]
    pub fn sections(&self) -> &[IniSection] {
        &self.sections
    }

    #[must_use]
    pub fn section(&self, name: &str) -> Option<&IniSection> {
        self.sections
            .iter()
            .find(|section| section.name.eq_ignore_ascii_case(name))
    }

    pub fn sections_named<'a>(
        &'a self,
        name: &'a str,
    ) -> impl DoubleEndedIterator<Item = &'a IniSection> + 'a {
        self.sections
            .iter()
            .filter(move |section| section.name.eq_ignore_ascii_case(name))
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[CatalogDiagnostic] {
        &self.diagnostics
    }
}

impl IdPathCatalog {
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
        let mut catalog = Self::default();

        for (index, raw_line) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty()
                || line.starts_with(';')
                || line.starts_with('#')
                || (line.starts_with('[') && line.ends_with(']'))
            {
                continue;
            }

            let Some((raw_id, raw_path)) = line.split_once('=') else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: "expected id=virtual-path".to_owned(),
                });
                continue;
            };
            let Ok(id) = raw_id.trim().parse::<u32>() else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: format!("invalid numeric id: {}", raw_id.trim()),
                });
                continue;
            };
            let Ok(path) = VirtualPath::try_from(raw_path.trim()) else {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: format!("invalid virtual path: {}", raw_path.trim()),
                });
                continue;
            };

            if catalog.entries.insert(id, path).is_some() {
                catalog.diagnostics.push(CatalogDiagnostic {
                    line: line_number,
                    detail: format!("id {id} replaces an earlier entry"),
                });
            }
        }

        catalog
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&VirtualPath> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[CatalogDiagnostic] {
        &self.diagnostics
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (u32, &VirtualPath)> {
        self.entries.iter().map(|(id, path)| (*id, path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_indexes_every_valid_entry_without_hiding_bad_lines() {
        let catalog = IdPathCatalog::parse(
            b"; exact catalog\r\n100=c3\\body.c3\r\nbad=line\r\n200=texture.dds\r\n",
        );

        assert_eq!(catalog.len(), 2);
        assert_eq!(
            catalog.get(100).map(VirtualPath::as_str),
            Some("c3/body.c3")
        );
        assert_eq!(
            catalog.get(200).map(VirtualPath::as_str),
            Some("texture.dds")
        );
        assert_eq!(catalog.diagnostics().len(), 1);
        assert_eq!(catalog.diagnostics()[0].line, 3);
    }

    #[test]
    fn section_parser_retains_repeated_properties_and_reports_orphans() {
        let catalog = SectionCatalog::parse(
            b"orphan=yes\r\n[410000]\r\nPart=2\r\nMesh0=410000\r\nMesh0=410001\r\n",
        );

        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.sections()[0].name, "410000");
        assert_eq!(catalog.sections()[0].first("part"), Some("2"));
        assert_eq!(catalog.sections()[0].properties().len(), 3);
        assert_eq!(catalog.diagnostics().len(), 1);
        assert_eq!(catalog.diagnostics()[0].line, 1);
    }

    #[test]
    fn named_section_iterator_preserves_duplicate_authoring_order() {
        let catalog = SectionCatalog::parse(
            b"[MagicSkillType4000]\r\nFrame0=missing.dds\r\n[other]\r\nFrame0=x.dds\r\n[magicskilltype4000]\r\nFrame0=valid.dds\r\n",
        );
        let frames = catalog
            .sections_named("MagicSkillType4000")
            .filter_map(|section| section.first("Frame0"))
            .collect::<Vec<_>>();
        assert_eq!(frames, ["missing.dds", "valid.dds"]);
        assert_eq!(
            catalog
                .sections_named("MagicSkillType4000")
                .next_back()
                .and_then(|section| section.first("Frame0")),
            Some("valid.dds")
        );
    }

    #[test]
    fn section_parser_accepts_tq_double_slash_comments() {
        let catalog = SectionCatalog::parse(
            b"// exact TQ catalogue comment\r\n[Item710001]\r\nFrameAmount=1\r\nFrame0=data/ItemMinIcon/710001.dds\r\n",
        );

        assert!(catalog.diagnostics().is_empty());
        assert_eq!(
            catalog
                .section("Item710001")
                .and_then(|section| section.first("Frame0")),
            Some("data/ItemMinIcon/710001.dds")
        );
    }
}
