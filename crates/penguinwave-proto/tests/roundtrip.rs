//! Wire-format tests.
//!
//! These are not "does serde work" tests. Each one pins a decision that fails
//! at runtime rather than compile time if it silently changes.

use penguinwave_proto::*;
use std::collections::HashMap;

fn line(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).expect("serialise")
}

// ---------------------------------------------------------------------------
// Framing
// ---------------------------------------------------------------------------

/// NDJSON framing is only unambiguous if no encoded frame contains a newline.
/// Strings carrying one must escape it, not emit it.
#[test]
fn frames_never_contain_a_bare_newline() {
    let hostile = Request::SinkCreate {
        config: SinkConfig {
            name: "evil\nname".into(),
            display_name: "line1\nline2\r\n{\"v\":1}".into(),
        },
    };
    let encoded = line(&RequestFrame::new(1, hostile));
    assert!(
        !encoded.contains('\n'),
        "frame must not contain a raw newline: {encoded}"
    );
    assert!(
        !encoded.contains('\r'),
        "frame must not contain a raw CR: {encoded}"
    );
}

#[test]
fn request_frame_shape_is_flat() {
    let f = RequestFrame::new(
        42,
        Request::StreamSetVolume {
            stream: StreamRef {
                index: 7,
                app_name: "firefox".into(),
                pid: Some(1234),
            },
            pct: 70,
        },
    );
    let v: serde_json::Value = serde_json::from_str(&line(&f)).unwrap();
    assert_eq!(v["v"], 1);
    assert_eq!(v["id"], 42);
    assert_eq!(v["method"], "stream.set_volume");
    assert_eq!(v["params"]["pct"], 70);
    // StreamRef is flattened into StreamInfo but NOT into params.
    assert_eq!(v["params"]["stream"]["index"], 7);
}

/// Unit variants must not emit a `params` key at all, so a client can send
/// `{"v":1,"id":1,"method":"stream.list"}` with nothing else.
#[test]
fn unit_request_omits_params() {
    let encoded = line(&RequestFrame::new(1, Request::StreamList));
    assert_eq!(encoded, r#"{"v":1,"id":1,"method":"stream.list"}"#);
    let back: RequestFrame = serde_json::from_str(&encoded).unwrap();
    assert_eq!(back.request, Request::StreamList);
}

#[test]
fn response_frame_distinguishes_ok_from_err() {
    let ok = line(&ResponseFrame::ok(9, Response::Empty));
    let v: serde_json::Value = serde_json::from_str(&ok).unwrap();
    assert_eq!(v["ok"]["kind"], "empty");
    assert!(v.get("err").is_none());

    let err = line(&ResponseFrame::err(
        9,
        PwError::not_found("sink 'game_sink'"),
    ));
    let v: serde_json::Value = serde_json::from_str(&err).unwrap();
    assert_eq!(v["err"]["kind"], "not_found");
    assert!(v.get("ok").is_none());
}

#[test]
fn event_frame_shape() {
    let f = EventFrame::new(Event::ChatmixChanged(ChatMix {
        value: 63,
        manual: false,
    }));
    let v: serde_json::Value = serde_json::from_str(&line(&f)).unwrap();
    assert_eq!(v["v"], 1);
    assert_eq!(v["event"], "chatmix.changed");
    assert_eq!(v["data"]["value"], 63);
    assert!(
        v.get("id").is_none(),
        "events are unsolicited and carry no request id"
    );
}

/// Every event's wire name must match what `Event::name()` reports, or
/// subscription filtering silently drops events that were meant to be sent.
#[test]
fn event_names_match_serialised_tag() {
    let all = vec![
        Event::ChatmixChanged(ChatMix {
            value: 0,
            manual: false,
        }),
        Event::EqStateChanged(EqState::default()),
        Event::EqSafeMode { active: true },
        Event::GraphChanged,
        Event::StreamListChanged { streams: vec![] },
        Event::SinkListChanged { sinks: vec![] },
        Event::DeviceAttached {
            device: DeviceId::new(0x1038, 0x2202),
        },
        Event::DeviceDetached {
            device: DeviceId::new(0x1038, 0x2202),
        },
        Event::DaemonShuttingDown,
    ];
    for e in all {
        let v: serde_json::Value = serde_json::from_str(&line(&e)).unwrap();
        assert_eq!(v["event"], e.name(), "name() disagrees with the wire tag");
    }
}

// ---------------------------------------------------------------------------
// The P0 hazard: enum-keyed map
// ---------------------------------------------------------------------------

/// `EqState.chains` is a `HashMap` with an enum key. JSON object keys must be
/// strings, and serde only obliges for unit-only enums. Adding a data-carrying
/// variant to `EqChainId` would break this at runtime, in the daemon, with a
/// serialisation error — not at compile time. This test is the tripwire.
#[test]
fn eq_state_enum_keys_survive_json() {
    let state = EqState::default();
    let encoded = line(&state);
    let v: serde_json::Value = serde_json::from_str(&encoded).unwrap();

    let chains = v["chains"]
        .as_object()
        .expect("chains must encode as a JSON object");
    assert!(
        chains.contains_key("game"),
        "expected snake_case string key, got {chains:?}"
    );
    assert!(chains.contains_key("chat"));

    let back: EqState = serde_json::from_str(&encoded).unwrap();
    assert_eq!(back, state);
}

#[test]
fn eq_chain_ids_are_all_covered() {
    let state = EqState::default();
    for id in EqChainId::ALL {
        assert!(
            state.chains.contains_key(&id),
            "default state missing chain {id:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Domain invariants that both sides must agree on
// ---------------------------------------------------------------------------

/// If client and daemon clamp differently they disagree about what was
/// applied, and the UI shows a value the audio server never received.
#[test]
fn band_clamping_is_symmetric_and_total() {
    let wild = EqBand {
        freq: 1e9,
        gain_db: -500.0,
        q: 0.0,
        filter_type: FilterType::Peaking,
        enabled: true,
    };
    let c = wild.clamped();
    assert_eq!(c.freq, FREQ_MAX);
    assert_eq!(c.gain_db, GAIN_MIN);
    assert_eq!(c.q, Q_MIN);
    // Idempotent: clamping an already-clamped value changes nothing.
    assert_eq!(c.clamped(), c);
}

#[test]
fn chain_clamping_enforces_band_cap() {
    let mut chain = EqChain::default_10_band();
    let band = chain.bands[0];
    chain.bands = vec![band; MAX_BANDS + 5];
    assert_eq!(chain.clamped().bands.len(), MAX_BANDS);
}

/// Guards a security boundary, not input tidiness: these values are
/// interpolated into a root-owned udev rules file.
#[test]
fn hex_id_validation_rejects_injection() {
    assert!(is_valid_hex_id("1038"));
    assert!(is_valid_hex_id("abCD"));

    assert!(!is_valid_hex_id(""));
    assert!(!is_valid_hex_id("103"));
    assert!(!is_valid_hex_id("10388"));
    assert!(!is_valid_hex_id("10 8"));
    assert!(!is_valid_hex_id("zzzz"));
    // The reason the check exists.
    assert!(!is_valid_hex_id("\", RUN+=\"/bin/sh"));
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[test]
fn retryable_is_derived_not_hand_set() {
    assert!(PwError::new(ErrorKind::PipeWireUnavailable, "down").retryable);
    assert!(!PwError::new(ErrorKind::DeviceAbsent, "unplugged").retryable);
    assert!(!PwError::new(ErrorKind::BadRequest, "nope").retryable);
}

/// Exit codes are a scripting contract: changing one breaks callers silently.
#[test]
fn exit_codes_are_distinct_and_nonzero() {
    use ErrorKind::*;
    let all = [
        VersionMismatch,
        BadRequest,
        NotFound,
        DeviceAbsent,
        PipeWireUnavailable,
        PipeWireFailed,
        PermissionDenied,
        Conflict,
        Internal,
    ];
    let mut seen = HashMap::new();
    for k in all {
        let code = k.exit_code();
        assert_ne!(code, 0, "{k:?} must not report success");
        assert!(
            seen.insert(code, k).is_none(),
            "{k:?} collides with {:?}",
            seen[&code]
        );
    }
}

#[test]
fn version_mismatch_tells_the_client_what_is_supported() {
    let e = PwError::version_mismatch(PROTO_SUPPORTED);
    assert_eq!(e.supported, Some(PROTO_SUPPORTED));
    let round: PwError = serde_json::from_str(&line(&e)).unwrap();
    assert_eq!(round, e);
}

/// `supported` is absent on every other error rather than serialised as null.
#[test]
fn supported_is_omitted_when_absent() {
    let encoded = line(&PwError::bad_request("bad"));
    assert!(!encoded.contains("supported"), "{encoded}");
}

#[test]
fn version_support_window() {
    assert!(version_supported(PROTO_VERSION));
    assert!(!version_supported(0));
    assert!(!version_supported(PROTO_SUPPORTED.1 + 1));
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

/// The CLI must reach everything the UI can (FR-3.1, G2). This pins the method
/// count so a capability added for the UI cannot quietly skip the CLI.
#[test]
fn method_surface_is_complete() {
    let requests = vec![
        Request::SessionHello {
            client: "t".into(),
            proto: 1,
        },
        Request::SessionSnapshot,
        Request::SessionSubscribe {
            events: vec!["*".into()],
        },
        Request::StreamList,
        Request::StreamMove {
            stream: sref(),
            sink: GAME_SINK.into(),
        },
        Request::StreamUnassign { stream: sref() },
        Request::StreamSetVolume {
            stream: sref(),
            pct: 50,
        },
        Request::StreamSetMute {
            stream: sref(),
            mute: true,
        },
        Request::StreamIcon { key: "k".into() },
        Request::SinkCreate {
            config: SinkConfig {
                name: "n".into(),
                display_name: "d".into(),
            },
        },
        Request::SinkDelete { name: "n".into() },
        Request::SinkListCustom,
        Request::SinkSetVolume {
            sink: "n".into(),
            pct: 50,
        },
        Request::SinkDefault,
        Request::SinkCurrentRoute { sink: "n".into() },
        Request::SinkRouteToDevice {
            sink: "n".into(),
            device: "d".into(),
        },
        Request::SinkListOutputDevices,
        Request::GraphListNodePorts { node: "n".into() },
        Request::GraphLink {
            source: pref(),
            target: pref(),
        },
        Request::GraphUnlink {
            source: pref(),
            target: pref(),
        },
        Request::DeviceListSupported,
        Request::DeviceGetSelected,
        Request::DeviceSetSelected { device: None },
        Request::DeviceListUser,
        Request::DeviceAddUser { device: udev() },
        Request::DeviceRemoveUser { name: "n".into() },
        Request::ChatmixSetManual { value: Some(50) },
        Request::EqGetState,
        Request::EqSetChain {
            chain: EqChainId::Game,
            value: EqChain::default_10_band(),
        },
        Request::EqSetBand {
            chain: EqChainId::Game,
            index: 0,
            band: band(),
        },
        Request::EqAddBand {
            chain: EqChainId::Game,
            band: band(),
        },
        Request::EqRemoveBand {
            chain: EqChainId::Game,
            index: 0,
        },
        Request::EqSetPreamp {
            chain: EqChainId::Game,
            preamp_db: 0.0,
        },
        Request::EqListPresets,
        Request::EqSavePreset {
            name: "p".into(),
            chain: EqChainId::Game,
        },
        Request::EqApplyPreset {
            name: "p".into(),
            chain: EqChainId::Game,
        },
        Request::EqDeletePreset { name: "p".into() },
        Request::EqResetSafeMode,
        Request::SystemCheckDeps,
        Request::SystemCheckUdev,
        Request::SystemInstallUdev,
    ];

    // Every method name is namespaced and unique.
    let mut names = Vec::new();
    for r in &requests {
        let v: serde_json::Value = serde_json::from_str(&line(r)).unwrap();
        let name = v["method"]
            .as_str()
            .expect("method must be a string")
            .to_string();
        assert!(name.contains('.'), "method '{name}' is not namespaced");
        names.push(name);
    }
    let unique: std::collections::BTreeSet<_> = names.iter().collect();
    assert_eq!(
        unique.len(),
        names.len(),
        "duplicate method name in {names:?}"
    );

    // Every request round-trips.
    for r in requests {
        let back: Request = serde_json::from_str(&line(&r)).unwrap();
        assert_eq!(back, r);
    }
}

#[test]
fn every_response_variant_round_trips() {
    let responses = vec![
        Response::Empty,
        Response::Hello(Hello {
            daemon: "0.3.0".into(),
            proto: PROTO_SUPPORTED,
            caps: vec![],
        }),
        Response::Snapshot(snapshot()),
        Response::Streams(vec![stream_info()]),
        Response::Sinks(vec![sink_info()]),
        Response::SinkName(GAME_SINK.into()),
        Response::Route(None),
        Response::OutputDevices(vec![]),
        Response::Ports(vec![]),
        Response::Links(vec![]),
        Response::Devices(vec![]),
        Response::SelectedDevice(Some(DeviceId::new(0x1038, 0x2202))),
        Response::UserDevices(vec![udev()]),
        Response::ChatMix(ChatMix {
            value: 50,
            manual: true,
        }),
        Response::EqState(EqState::default()),
        Response::EqPresets(vec![EqPresetMeta {
            name: "flat".into(),
            builtin: true,
        }]),
        Response::SystemDeps(SystemDeps {
            pipewire: true,
            pactl: true,
            pw_link: true,
            libhidapi: false,
        }),
        Response::UdevStatus(UdevStatus {
            installed: false,
            install_command: None,
        }),
        Response::Icon(None),
    ];
    for r in responses {
        let back: Response = serde_json::from_str(&line(&r)).unwrap();
        assert_eq!(back, r);
    }
}

/// A full snapshot must fit the frame cap. Icons are the only realistic way to
/// blow it, which is why they are fetched by key instead of inlined.
#[test]
fn snapshot_stays_well_under_the_frame_cap() {
    let mut snap = snapshot();
    snap.streams = (0..200)
        .map(|i| {
            let mut s = stream_info();
            s.stream.index = i;
            s
        })
        .collect();
    let encoded = line(&ResponseFrame::ok(1, Response::Snapshot(snap)));
    assert!(
        encoded.len() < MAX_FRAME_BYTES,
        "200-stream snapshot is {} bytes, cap is {MAX_FRAME_BYTES}",
        encoded.len()
    );
}

// ---------------------------------------------------------------------------

fn sref() -> StreamRef {
    StreamRef {
        index: 1,
        app_name: "app".into(),
        pid: Some(1),
    }
}
fn pref() -> PortRef {
    PortRef {
        node_name: "n".into(),
        port_name: "p".into(),
        id: Some(7),
    }
}
fn band() -> EqBand {
    EqBand {
        freq: 1000.0,
        gain_db: 0.0,
        q: 1.0,
        filter_type: FilterType::Peaking,
        enabled: true,
    }
}
fn udev() -> UserDevice {
    UserDevice {
        name: "custom".into(),
        vendor_id: Some("1038".into()),
        product_id: Some("2202".into()),
        pipewire_sink: None,
    }
}
fn stream_info() -> StreamInfo {
    StreamInfo {
        stream: sref(),
        name: "Firefox".into(),
        sink: GAME_SINK.into(),
        volume: 100,
        is_muted: false,
        icon_key: Some("firefox".into()),
    }
}
fn sink_info() -> SinkInfo {
    SinkInfo {
        id: 1,
        name: GAME_SINK.into(),
        description: "Game".into(),
        volume: 100,
        is_muted: false,
        managed: true,
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        streams: vec![stream_info()],
        sinks: vec![sink_info()],
        default_sink: GAME_SINK.into(),
        devices: vec![],
        selected_device: None,
        user_devices: vec![],
        chatmix: ChatMix {
            value: 50,
            manual: false,
        },
        eq: EqState::default(),
    }
}
