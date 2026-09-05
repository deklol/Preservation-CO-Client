// Source: @digitalm1nd on x.com / _dek on Discord - Preservation Conquer project - https://discord.gg/CvKPXEHYRY
use crate::{
    action::Action,
    assets::{Assets, Result},
    character::Character,
    movement::Player,
};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Source, buffer::SamplesBuffer};
use sandbox_data::audio::{ActionSoundCatalog, ActionSoundKey};
use std::{collections::HashMap, io::Cursor};

pub struct MovementAudio {
    device: Option<MixerDeviceSink>,
    paths: [Option<String>; 6],
    clips: HashMap<String, SamplesBuffer>,
    voices: HashMap<String, rodio::Player>,
    last_action: Option<(Action, u64)>,
}

impl MovementAudio {
    pub fn load(assets: &mut Assets, character: &Character) -> Result<Self> {
        let catalog = ActionSoundCatalog::parse(
            &assets.read("ini/sound.ini").unwrap_or_default(),
            &assets.read("ini/ActionSound.ini")?,
        );
        let paths = Action::ALL.map(|action| {
            action_sound(&catalog, character.body_id, character.weapon_action, action)
                .map(str::to_owned)
        });
        let mut clips = HashMap::new();
        for path in paths.iter().flatten() {
            if clips.contains_key(path) {
                continue;
            }
            let decoded = assets.read(path).and_then(decode_audio_bytes);
            match decoded {
                Ok(clip) => {
                    clips.insert(path.clone(), clip);
                }
                Err(error) => eprintln!("Audio: {path}: {error}"),
            }
        }
        Ok(Self {
            device: None,
            paths,
            clips,
            voices: HashMap::new(),
            last_action: None,
        })
    }

    pub fn open_device(&mut self) {
        match DeviceSinkBuilder::from_default_device()
            .and_then(|builder| builder.open_sink_or_fallback())
        {
            Ok(mut device) => {
                device.log_on_drop(false);
                self.device = Some(device);
            }
            Err(error) => eprintln!("Audio unavailable: {error}"),
        }
    }

    pub fn update(&mut self, player: &Player) {
        let action = (player.action, player.action_instance);
        if self.last_action == Some(action) {
            return;
        }
        self.last_action = Some(action);
        let Some(device) = &self.device else { return };
        let Some(path) = &self.paths[player.action.index()] else {
            return;
        };
        let Some(clip) = self.clips.get(path) else {
            return;
        };
        if let Some(previous) = self.voices.remove(path) {
            previous.stop();
        }
        let voice = rodio::Player::connect_new(device.mixer());
        voice.set_volume(0.8);
        voice.append(clip.clone());
        self.voices.insert(path.clone(), voice);
    }

    pub fn check(&self) -> Result<()> {
        for path in self.paths.iter().flatten() {
            if !self.clips.contains_key(path) {
                return Err(format!("Could not decode movement sound {path}").into());
            }
        }
        println!("Audio OK: {} decoded movement sounds", self.clips.len());
        Ok(())
    }
}

fn action_sound(
    catalog: &ActionSoundCatalog,
    body_id: u32,
    weapon_id: u32,
    action: Action,
) -> Option<&str> {
    let action_id = action.catalog_id();
    catalog
        .get(ActionSoundKey {
            body_id,
            weapon_id,
            action_id,
        })
        .or_else(|| {
            catalog.get(ActionSoundKey {
                body_id,
                weapon_id: 999,
                action_id,
            })
        })
}

fn decode_audio_bytes(bytes: Vec<u8>) -> Result<SamplesBuffer> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Audio exceeds 32 MiB safety limit".into());
    }
    let decoder = Decoder::try_from(Cursor::new(bytes))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples = decoder.collect::<Vec<_>>();
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipped_sound_precedes_generic_and_absence_stays_silent() {
        let catalog = ActionSoundCatalog::parse_action_sound(
            b"3.611.130=sound/equipped.wav\n3.999.130=sound/jump.wav\n3.999.120=sound/runL.wav\n3.999.121=sound/runR.wav\n"
        );
        assert_eq!(
            action_sound(&catalog, 3, 611, Action::Jump),
            Some("sound/equipped.wav")
        );
        assert_eq!(
            action_sound(&catalog, 3, 410, Action::Jump),
            Some("sound/jump.wav")
        );
        assert_eq!(
            action_sound(&catalog, 3, 611, Action::RunLeft),
            Some("sound/runl.wav")
        );
        assert_eq!(
            action_sound(&catalog, 3, 611, Action::RunRight),
            Some("sound/runr.wav")
        );
        assert_eq!(action_sound(&catalog, 3, 611, Action::Idle), None);
        assert!(decode_audio_bytes(vec![0; 16]).is_err());
    }
}
