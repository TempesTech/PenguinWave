//! Parametric EQ: bands, chains, presets.
//!
//! Lifted from the pre-split `eq/model.rs`, which was already `serde`-derived
//! and free of Tauri types. The clamping helpers come along because they
//! express range invariants that both sides of the socket must agree on: a
//! client that clamps differently to the daemon produces silent disagreement
//! about what was actually applied.
//!
//! Filter-chain *node names* deliberately do not live here. They are a detail
//! of how the backend realises a chain in PipeWire, not part of the contract
//! between daemon and client.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Fixed pool size per chain; unused slots run identity coefficients.
pub const MAX_BANDS: usize = 16;

pub const FREQ_MIN: f32 = 20.0;
pub const FREQ_MAX: f32 = 20_000.0;
pub const GAIN_MIN: f32 = -24.0;
pub const GAIN_MAX: f32 = 24.0;
pub const Q_MIN: f32 = 0.1;
pub const Q_MAX: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EqBand {
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
    pub filter_type: FilterType,
    pub enabled: bool,
}

impl EqBand {
    pub fn clamped(mut self) -> Self {
        self.freq = self.freq.clamp(FREQ_MIN, FREQ_MAX);
        self.gain_db = self.gain_db.clamp(GAIN_MIN, GAIN_MAX);
        self.q = self.q.clamp(Q_MIN, Q_MAX);
        self
    }
}

/// Which of the two managed chains a request refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EqChainId {
    Game,
    Chat,
}

impl EqChainId {
    pub const ALL: [EqChainId; 2] = [EqChainId::Game, EqChainId::Chat];

    pub fn description(&self) -> &'static str {
        match self {
            EqChainId::Game => "PenguinWave EQ (Game)",
            EqChainId::Chat => "PenguinWave EQ (Chat)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EqChain {
    pub enabled: bool,
    pub preamp_db: f32,
    pub bands: Vec<EqBand>,
}

impl EqChain {
    /// 10 flat peaking bands at ISO centre frequencies — the "graphic EQ" default.
    pub fn default_10_band() -> Self {
        const ISO_FREQS: [f32; 10] = [
            31.0, 62.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
        ];
        EqChain {
            enabled: true,
            preamp_db: 0.0,
            bands: ISO_FREQS
                .iter()
                .map(|&freq| EqBand {
                    freq,
                    gain_db: 0.0,
                    q: 1.1,
                    filter_type: FilterType::Peaking,
                    enabled: true,
                })
                .collect(),
        }
    }

    pub fn clamped(mut self) -> Self {
        self.preamp_db = self.preamp_db.clamp(GAIN_MIN, GAIN_MAX);
        self.bands.truncate(MAX_BANDS);
        self.bands = self.bands.into_iter().map(EqBand::clamped).collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EqState {
    /// Keyed by chain. `EqChainId` is a unit-only enum, so it serialises as a
    /// JSON string and is legal as an object key; the round-trip is pinned by
    /// a test because a non-string key would fail at runtime, not compile time.
    pub chains: HashMap<EqChainId, EqChain>,
    /// Set when the filter chain failed to load and audio was routed around
    /// the EQ to keep sound working. Persisted on disk so it survives a daemon
    /// crash, which is the case it exists for.
    pub safe_mode: bool,
}

impl Default for EqState {
    fn default() -> Self {
        let mut chains = HashMap::new();
        for id in EqChainId::ALL {
            chains.insert(id, EqChain::default_10_band());
        }
        EqState {
            chains,
            safe_mode: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EqPreset {
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    pub chain: EqChain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EqPresetMeta {
    pub name: String,
    pub builtin: bool,
}
