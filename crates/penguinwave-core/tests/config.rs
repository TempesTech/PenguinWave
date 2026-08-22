//! The 0.3.0 layout is the 0.2.0 layout: same directory, same file names,
//! same JSON shape. These tests hold that claim in place instead of a
//! migration step.

mod common;
use common::TempDir;

use penguinwave_core::ConfigStore;
use penguinwave_proto::EqChainId;

/// Written by 0.2.0, with `safe_mode` in the file rather than a marker.
const V0_2_STATE: &str = r#"{
  "chains": {
    "game": { "enabled": true, "preamp_db": -3.5,
      "bands": [ { "freq": 120.0, "gain_db": 4.0, "q": 1.1, "filter_type": "peaking", "enabled": true } ] },
    "chat": { "enabled": false, "preamp_db": 0.0, "bands": [] }
  },
  "safe_mode": false
}"#;

const V0_2_DEVICES: &str = r#"[[devices]]
name = "My Headset"
vendor_id = "1038"
product_id = "2202"
"#;

fn seed(dir: &TempDir) -> ConfigStore {
    std::fs::create_dir_all(dir.0.join("eq")).unwrap();
    std::fs::write(dir.0.join("eq/state.json"), V0_2_STATE).unwrap();
    std::fs::write(dir.0.join("devices.toml"), V0_2_DEVICES).unwrap();
    ConfigStore::new(&dir.0)
}

#[test]
fn a_v0_2_eq_state_loads_unchanged() {
    let dir = TempDir::new("v02state");
    let store = seed(&dir);

    let state = store.load_eq_state();
    let game = &state.chains[&EqChainId::Game];
    assert_eq!(game.preamp_db, -3.5);
    assert_eq!(game.bands.len(), 1);
    assert_eq!(game.bands[0].freq, 120.0);
    assert!(!state.chains[&EqChainId::Chat].enabled);
}

#[test]
fn a_v0_2_devices_file_loads_unchanged() {
    let dir = TempDir::new("v02devices");
    let store = seed(&dir);

    let devices = store.load_user_devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name, "My Headset");
    assert_eq!(devices[0].vendor_id.as_deref(), Some("1038"));
}

#[test]
fn safe_mode_comes_from_the_marker_not_the_state_file() {
    let dir = TempDir::new("marker");
    let store = seed(&dir);
    assert!(!store.load_eq_state().safe_mode);

    store.set_safe_mode(true).unwrap();
    assert!(store.load_eq_state().safe_mode);

    store.set_safe_mode(false).unwrap();
    assert!(!store.load_eq_state().safe_mode);
}

#[test]
fn a_corrupt_state_file_falls_back_to_defaults_rather_than_failing() {
    let dir = TempDir::new("corrupt");
    std::fs::create_dir_all(dir.0.join("eq")).unwrap();
    std::fs::write(dir.0.join("eq/state.json"), "{ not json").unwrap();

    let state = ConfigStore::new(&dir.0).load_eq_state();
    assert_eq!(state.chains.len(), EqChainId::ALL.len());
}

#[test]
fn writes_are_atomic_and_leave_no_temp_file() {
    let dir = TempDir::new("atomic");
    let store = ConfigStore::new(&dir.0);
    store.save_eq_state(&store.load_eq_state()).unwrap();

    let leftovers: Vec<_> = std::fs::read_dir(store.eq_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
        .collect();
    assert!(leftovers.is_empty());
}
