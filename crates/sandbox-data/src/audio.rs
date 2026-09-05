// Source: @digitalm1nd on x.com / _dek on Discord - Preservation Conquer project - https://discord.gg/CvKPXEHYRY
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActionSoundKey {
    pub body_id: u32,
    pub weapon_id: u32,
    pub action_id: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionSoundEntry {
    pub key: ActionSoundKey,
    pub content_key: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioCatalogDiagnostic {
    pub source: &'static str,
    pub line: usize,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct ActionSoundCatalog {
    entries: HashMap<ActionSoundKey, ActionSoundEntry>,
    diagnostics: Vec<AudioCatalogDiagnostic>,
}

impl ActionSoundCatalog {
    #[must_use]
    pub fn parse(sound_ini: &[u8], action_sound_ini: &[u8]) -> Self {
        let mut catalog = Self::default();
        catalog.parse_file("sound.ini", sound_ini);
        catalog.parse_file("ActionSound.ini", action_sound_ini);
        catalog
    }

    #[must_use]
    pub fn parse_action_sound(bytes: &[u8]) -> Self {
        Self::parse(&[], bytes)
    }

    fn parse_file(&mut self, source: &'static str, bytes: &[u8]) {
        let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
        for (index, raw_line) in text.lines().enumerate() {
            let line = index + 1;
            let line_text = raw_line.trim();
            if line_text.is_empty() || line_text.starts_with(';') || line_text.starts_with('#') {
                continue;
            }
            let Some((raw_key, raw_value)) = line_text.split_once('=') else {
                self.diagnostics.push(AudioCatalogDiagnostic {
                    source,
                    line,
                    detail: "expected body.weapon.action=path".to_owned(),
                });
                continue;
            };
            let parts = raw_key.trim().split('.').collect::<Vec<_>>();
            if parts.len() != 3 {
                self.diagnostics.push(AudioCatalogDiagnostic {
                    source,
                    line,
                    detail: format!("expected three numeric key components: {}", raw_key.trim()),
                });
                continue;
            }
            let Some(body_id) = parse_id(parts[0]) else {
                self.invalid_key(source, line, raw_key.trim(), "body id");
                continue;
            };
            let Some(weapon_id) = parse_id(parts[1]) else {
                self.invalid_key(source, line, raw_key.trim(), "weapon id");
                continue;
            };
            let Some(action_id) = parse_id(parts[2]) else {
                self.invalid_key(source, line, raw_key.trim(), "action id");
                continue;
            };
            let value = raw_value.trim();
            if is_absent(value) {
                continue;
            }
            let Some(content_key) = normalize_content_key(value) else {
                self.diagnostics.push(AudioCatalogDiagnostic {
                    source,
                    line,
                    detail: format!("invalid sound virtual path: {value}"),
                });
                continue;
            };
            let key = ActionSoundKey {
                body_id,
                weapon_id,
                action_id,
            };
            if self
                .entries
                .insert(
                    key,
                    ActionSoundEntry {
                        key,
                        content_key,
                        line,
                    },
                )
                .is_some()
            {
                self.diagnostics.push(AudioCatalogDiagnostic {
                    source,
                    line,
                    detail: format!("duplicate action tuple {body_id}.{weapon_id}.{action_id}"),
                });
            }
        }
    }

    fn invalid_key(&mut self, source: &'static str, line: usize, key: &str, component: &str) {
        self.diagnostics.push(AudioCatalogDiagnostic {
            source,
            line,
            detail: format!("invalid {component} in {key}"),
        });
    }

    #[must_use]
    pub fn get(&self, key: ActionSoundKey) -> Option<&str> {
        self.entries
            .get(&key)
            .map(|entry| entry.content_key.as_str())
    }

    #[must_use]
    pub fn entry(&self, key: ActionSoundKey) -> Option<&ActionSoundEntry> {
        self.entries.get(&key)
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
    pub fn diagnostics(&self) -> &[AudioCatalogDiagnostic] {
        &self.diagnostics
    }
}

pub fn normalize_content_key(value: &str) -> Option<String> {
    let normalized = value.trim().replace('\\', "/").to_ascii_lowercase();
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "..")
    {
        None
    } else {
        Some(normalized)
    }
}

fn parse_id(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn is_absent(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("null") || value.trim().eq_ignore_ascii_case("none")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_catalog_normalizes_paths_and_absence() {
        let catalog = ActionSoundCatalog::parse_action_sound(
            b"1.999.130=Sound\\Jump.WAV\n1.999.131=none\n2.bad.130=x.wav\n",
        );
        assert_eq!(catalog.len(), 1);
        assert_eq!(
            catalog.get(ActionSoundKey {
                body_id: 1,
                weapon_id: 999,
                action_id: 130
            }),
            Some("sound/jump.wav")
        );
        assert_eq!(catalog.diagnostics().len(), 1);
    }

    #[test]
    fn action_sound_overrides_sound_ini() {
        let catalog =
            ActionSoundCatalog::parse(b"3.999.130=sound/old.wav", b"3.999.130=sound/jump.wav");
        assert_eq!(
            catalog.get(ActionSoundKey {
                body_id: 3,
                weapon_id: 999,
                action_id: 130
            }),
            Some("sound/jump.wav")
        );
        assert_eq!(catalog.diagnostics().len(), 1);
    }
}
