//! Compiled-in builtins plus user presets in `presets.json`.

use crate::domain::config::ConfigStore;
use crate::error::{CoreError, Result};
use penguinwave_proto::{EqBand, EqChain, EqPreset, EqPresetMeta, FilterType};

fn band(filter_type: FilterType, freq: f32, gain_db: f32, q: f32) -> EqBand {
    EqBand {
        freq,
        gain_db,
        q,
        filter_type,
        enabled: true,
    }
}

pub fn builtin_presets() -> Vec<EqPreset> {
    let preset = |name: &str, preamp_db: f32, bands: Vec<EqBand>| EqPreset {
        name: name.to_string(),
        builtin: true,
        chain: EqChain {
            enabled: true,
            preamp_db,
            bands,
        },
    };

    vec![
        preset("Flat", 0.0, EqChain::default_10_band().bands),
        preset(
            "Bass Boost",
            -4.0,
            vec![
                band(FilterType::LowShelf, 100.0, 6.0, 0.707),
                band(FilterType::Peaking, 250.0, 2.0, 1.0),
            ],
        ),
        preset(
            "Vocal Clarity",
            -3.0,
            vec![
                band(FilterType::HighPass, 80.0, 0.0, 0.707),
                band(FilterType::Peaking, 2_500.0, 4.0, 1.2),
                band(FilterType::Peaking, 5_000.0, 2.0, 1.5),
            ],
        ),
        preset(
            "Treble",
            -3.0,
            vec![
                band(FilterType::HighShelf, 6_000.0, 5.0, 0.707),
                band(FilterType::Peaking, 10_000.0, 2.0, 1.0),
            ],
        ),
    ]
}

fn is_builtin(name: &str) -> bool {
    builtin_presets().iter().any(|p| p.name == name)
}

pub fn list(store: &ConfigStore) -> Vec<EqPresetMeta> {
    builtin_presets()
        .iter()
        .chain(store.load_user_presets().iter())
        .map(|p| EqPresetMeta {
            name: p.name.clone(),
            builtin: p.builtin,
        })
        .collect()
}

pub fn get(store: &ConfigStore, name: &str) -> Result<EqPreset> {
    builtin_presets()
        .into_iter()
        .chain(store.load_user_presets())
        .find(|p| p.name == name)
        .ok_or_else(|| CoreError::PresetNotFound(name.to_string()))
}

/// Save or overwrite a user preset. Builtin names are protected.
pub fn save(store: &ConfigStore, name: &str, chain: EqChain) -> Result<()> {
    if is_builtin(name) {
        return Err(CoreError::BuiltinPreset(name.to_string()));
    }
    if name.trim().is_empty() {
        return Err(CoreError::Invalid("preset name is empty".into()));
    }
    let mut presets = store.load_user_presets();
    presets.retain(|p| p.name != name);
    presets.push(EqPreset {
        name: name.to_string(),
        builtin: false,
        chain: chain.clamped(),
    });
    store.save_user_presets(&presets)
}

pub fn delete(store: &ConfigStore, name: &str) -> Result<()> {
    if is_builtin(name) {
        return Err(CoreError::BuiltinPreset(name.to_string()));
    }
    let mut presets = store.load_user_presets();
    let before = presets.len();
    presets.retain(|p| p.name != name);
    if presets.len() == before {
        return Err(CoreError::PresetNotFound(name.to_string()));
    }
    store.save_user_presets(&presets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use penguinwave_proto::MAX_BANDS;

    #[test]
    fn builtins_are_valid() {
        for preset in builtin_presets() {
            assert!(preset.builtin);
            assert!(!preset.chain.bands.is_empty());
            assert!(preset.chain.bands.len() <= MAX_BANDS);
            assert_eq!(
                preset.chain.clone().clamped(),
                preset.chain,
                "preset {} out of range",
                preset.name
            );
        }
    }
}
