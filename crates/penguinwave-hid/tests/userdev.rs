use penguinwave_hid::userdev;
use penguinwave_proto::UserDevice;

fn dev(name: &str, vid: Option<&str>, pid: Option<&str>) -> UserDevice {
    UserDevice {
        name: name.into(),
        vendor_id: vid.map(Into::into),
        product_id: pid.map(Into::into),
        pipewire_sink: None,
    }
}

#[test]
fn valid_ids_round_trip_through_the_file() {
    let dir = std::env::temp_dir().join("penguinwave-userdev-roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("devices.toml");

    let devices = vec![dev("Nova 7", Some("1038"), Some("2202"))];
    userdev::save_to(&path, &devices).unwrap();
    assert_eq!(userdev::load_from(&path), devices);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_file_loads_as_empty() {
    let path = std::env::temp_dir().join("penguinwave-userdev-absent/devices.toml");
    let _ = std::fs::remove_file(&path);
    assert!(userdev::load_from(&path).is_empty());
}

#[test]
fn non_hex_ids_are_rejected() {
    for bad in ["10 38", "103", "10388", "1038\"", "zzzz", "0x10"] {
        assert!(
            userdev::validate(&dev("x", Some(bad), None)).is_err(),
            "accepted {bad:?}"
        );
        assert!(
            userdev::validate(&dev("x", None, Some(bad))).is_err(),
            "accepted {bad:?}"
        );
    }
}

#[test]
fn hex_ids_are_accepted_in_either_case() {
    assert!(userdev::validate(&dev("x", Some("1AbF"), Some("0000"))).is_ok());
}

#[test]
fn absent_ids_are_allowed_but_empty_names_are_not() {
    assert!(userdev::validate(&dev("Sink only", None, None)).is_ok());
    assert!(userdev::validate(&dev("  ", None, None)).is_err());
}

#[test]
fn saving_an_invalid_id_writes_nothing() {
    let dir = std::env::temp_dir().join("penguinwave-userdev-reject");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("devices.toml");

    assert!(userdev::save_to(&path, &[dev("bad", Some("nope"), None)]).is_err());
    assert!(!path.exists());
}
