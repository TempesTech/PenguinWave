use penguinwave_core::domain::config::ConfigStore;
use penguinwave_core::eq::nodes::sink_node_name;
use penguinwave_core::{EqManager, EventBus};
use penguinwave_pipewire::{MockBackend, PipeWireBackend};
use penguinwave_proto::{EqBand, EqChainId, FilterType, MAX_BANDS};
use std::sync::Arc;

mod common;
use common::TempDir;

struct Harness {
    manager: Arc<EqManager>,
    backend: Arc<MockBackend>,
    events: Arc<EventBus>,
    store: ConfigStore,
    _dir: TempDir,
}

fn harness(tag: &str) -> Harness {
    let dir = TempDir::new(tag);
    // The fixtures were captured with the EQ running, so the chain nodes are
    // already in the graph.
    let backend = Arc::new(MockBackend::new());
    let store = ConfigStore::new(&dir.0);
    let events = Arc::new(EventBus::new());
    let manager = EqManager::new(backend.clone(), store.clone(), events.clone());
    Harness {
        manager,
        backend,
        events,
        store,
        _dir: dir,
    }
}

fn band(freq: f32, gain_db: f32) -> EqBand {
    EqBand {
        freq,
        gain_db,
        q: 1.1,
        filter_type: FilterType::Peaking,
        enabled: true,
    }
}

#[test]
fn init_spawns_the_chain_and_pushes_every_node() {
    let h = harness("init");
    h.manager.init();

    let calls = h.backend.calls();
    assert_eq!(
        calls.iter().filter(|c| *c == "spawn_filter_chain").count(),
        1
    );
    // One batched push per chain: preamp plus every slot.
    let pushes: Vec<&String> = calls
        .iter()
        .filter(|c| c.starts_with("set_node_props"))
        .collect();
    assert_eq!(pushes.len(), EqChainId::ALL.len());
    assert!(pushes[0].contains(&format!("({} props)", (MAX_BANDS + 1) * 5)));
    assert!(h.manager.is_active());
}

#[test]
fn conf_is_written_before_the_chain_is_spawned() {
    let h = harness("conf");
    h.manager.init();

    let conf = std::fs::read_to_string(h.store.eq_conf_path()).unwrap();
    assert!(conf.contains("penguinwave_eq_game"));
    assert!(conf.contains("stream.capture.sink = true"));
}

#[test]
fn setting_a_band_pushes_only_that_node() {
    let h = harness("setband");
    h.manager.init();

    let node_id = h
        .backend
        .resolve_node_id(sink_node_name(EqChainId::Game))
        .unwrap();
    let before = h.backend.calls().len();

    h.manager
        .set_band(EqChainId::Game, 0, band(1_000.0, 6.0))
        .unwrap();

    let new: Vec<String> = h.backend.calls().into_iter().skip(before).collect();
    assert_eq!(new, vec![format!("set_node_props {node_id} (5 props)")]);
}

#[test]
fn removing_a_band_repushes_the_whole_chain() {
    let h = harness("removeband");
    h.manager.init();
    let before = h.backend.calls().len();

    h.manager.remove_band(EqChainId::Game, 0).unwrap();

    let new: Vec<String> = h.backend.calls().into_iter().skip(before).collect();
    assert_eq!(new.len(), 1);
    assert!(new[0].contains(&format!("({} props)", (MAX_BANDS + 1) * 5)));
}

#[test]
fn every_mutation_publishes_a_state_event() {
    let h = harness("events");
    h.manager.init();
    let rx = h.events.subscribe();

    h.manager.set_preamp(EqChainId::Game, -6.0).unwrap();
    h.manager.set_chain_enabled(EqChainId::Chat, false).unwrap();

    let names: Vec<&'static str> = rx.try_iter().map(|e| e.name()).collect();
    assert_eq!(names, ["eq.state_changed", "eq.state_changed"]);
}

#[test]
fn band_values_are_clamped_on_the_way_in() {
    let h = harness("clamp");
    h.manager.init();

    h.manager
        .set_band(EqChainId::Game, 0, band(999_999.0, 999.0))
        .unwrap();

    let state = h.manager.state();
    let stored = state.chains[&EqChainId::Game].bands[0];
    assert_eq!(stored.freq, penguinwave_proto::FREQ_MAX);
    assert_eq!(stored.gain_db, penguinwave_proto::GAIN_MAX);
}

#[test]
fn a_full_chain_refuses_more_bands() {
    let h = harness("full");
    h.manager.init();

    while h.manager.state().chains[&EqChainId::Game].bands.len() < MAX_BANDS {
        h.manager
            .add_band(EqChainId::Game, band(1_000.0, 0.0))
            .unwrap();
    }
    assert!(h
        .manager
        .add_band(EqChainId::Game, band(1_000.0, 0.0))
        .is_err());
}

#[test]
fn out_of_range_band_index_is_rejected() {
    let h = harness("index");
    h.manager.init();
    assert!(h
        .manager
        .set_band(EqChainId::Game, 99, band(1_000.0, 0.0))
        .is_err());
    assert!(h.manager.remove_band(EqChainId::Game, 99).is_err());
}

#[test]
fn a_failing_chain_start_enters_safe_mode_and_leaves_a_marker() {
    let h = harness("safemode");
    h.backend.set_unavailable(true);
    let rx = h.events.subscribe();

    h.manager.init();

    assert!(h.manager.state().safe_mode);
    assert!(h.store.safe_mode());
    assert!(!h.manager.is_active());
    assert_eq!(
        rx.try_iter().map(|e| e.name()).collect::<Vec<_>>(),
        ["eq.safe_mode"]
    );
}

#[test]
fn safe_mode_blocks_mutations_until_reset() {
    let h = harness("blocked");
    h.backend.set_unavailable(true);
    h.manager.init();

    assert!(h.manager.set_preamp(EqChainId::Game, 0.0).is_err());

    h.backend.set_unavailable(false);
    h.manager.reset_safe_mode().unwrap();

    assert!(!h.manager.state().safe_mode);
    assert!(!h.store.safe_mode());
    h.manager.set_preamp(EqChainId::Game, 0.0).unwrap();
}

#[test]
fn a_safe_mode_marker_on_disk_keeps_the_chain_from_starting() {
    let h = harness("marker");
    h.store.set_safe_mode(true).unwrap();

    let manager = EqManager::new(h.backend.clone(), h.store.clone(), h.events.clone());
    manager.init();

    assert!(manager.state().safe_mode);
    assert!(!h.backend.calls().iter().any(|c| c == "spawn_filter_chain"));
}

#[test]
fn shutdown_persists_state_and_restores_direct_routing() {
    let h = harness("shutdown");
    h.manager.init();
    h.manager.set_preamp(EqChainId::Game, -9.0).unwrap();

    h.manager.shutdown();

    assert!(!h.manager.is_active());
    let reloaded = h.store.load_eq_state();
    assert_eq!(reloaded.chains[&EqChainId::Game].preamp_db, -9.0);
    assert!(h
        .backend
        .calls()
        .iter()
        .any(|c| c.starts_with("link_ports")));
}

#[test]
fn presets_round_trip_and_builtins_are_protected() {
    let h = harness("presets");
    h.manager.init();

    h.manager.set_preamp(EqChainId::Game, -5.0).unwrap();
    h.manager.save_preset(EqChainId::Game, "Mine").unwrap();

    assert!(h.manager.list_presets().iter().any(|p| p.name == "Mine"));
    assert!(h.manager.save_preset(EqChainId::Game, "Flat").is_err());
    assert!(h.manager.delete_preset("Flat").is_err());

    h.manager.apply_preset(EqChainId::Chat, "Mine").unwrap();
    assert_eq!(h.manager.state().chains[&EqChainId::Chat].preamp_db, -5.0);

    h.manager.delete_preset("Mine").unwrap();
    assert!(h.manager.delete_preset("Mine").is_err());
}

#[test]
fn routing_the_eq_into_a_managed_node_is_refused() {
    let h = harness("feedback");
    h.manager.init();

    for node in ["game_sink", "chat_sink", "penguinwave_eq_game_out"] {
        assert!(
            h.manager.route_output(EqChainId::Game, node).is_err(),
            "accepted {node}"
        );
    }
}

const HEADSET_SINK: &str = "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo";

/// An orphaned filter-chain from a killed daemon can leave its EQ output
/// linked back into a managed sink (game_sink/chat_sink) instead of the real
/// device. A fresh daemon's `init()` must not treat that stale target as
/// trustworthy and give up -- it should fall back to a real output.
///
/// Built on an empty graph, not the captured fixture: that one already has
/// the EQ correctly wired to the headset, which would mask exactly the bug
/// this test exists to catch.
#[test]
fn init_recovers_from_a_stale_eq_output_pointed_at_a_managed_node() {
    use penguinwave_pipewire::{MockBackend, PipeWireBackend};
    use penguinwave_proto::PortRef;

    let dir = TempDir::new("staleorphan");
    let backend = Arc::new(MockBackend::empty());
    backend.add_sink(HEADSET_SINK);
    backend.add_node("penguinwave_eq_game", 9001);
    backend.add_node("penguinwave_eq_chat", 9002);

    let port = |node: &str, name: &str| PortRef {
        node_name: node.to_string(),
        port_name: name.to_string(),
        id: None,
    };
    // The orphan: the EQ output node already exists, wired into game_sink
    // instead of a real device, as it would be left by a previous daemon.
    backend
        .link_ports(
            &port("penguinwave_eq_game_out", "output_FL"),
            &port("game_sink", "playback_FL"),
        )
        .unwrap();

    let store = ConfigStore::new(&dir.0);
    let events = Arc::new(EventBus::new());
    let manager = EqManager::new(backend.clone(), store, events);
    manager.init();

    let targets: Vec<String> = backend
        .list_links()
        .unwrap()
        .into_iter()
        .filter(|l| l.source.node_name == "penguinwave_eq_game_out")
        .map(|l| l.target.node_name)
        .collect();

    assert!(
        targets.iter().any(|n| n == HEADSET_SINK),
        "the EQ never reached a real device, still pointing at: {targets:?}"
    );
    assert!(
        !targets.iter().any(|n| n == "game_sink"),
        "the stale feedback link into game_sink was never cleared: {targets:?}"
    );
}

/// `CoreState`'s graph-watch route reconciliation and this manager's own
/// chain-respawn path both wire the same chain, on different threads, with
/// no other coordination. Without a shared lock they can each resolve a
/// different target from a different snapshot of the graph and both
/// partially succeed, leaving the EQ output linked to two devices at once --
/// a real feedback loop seen live after a PipeWire crash. `wiring_lock()` is
/// the fix; this proves it actually serializes, not just that it compiles.
#[test]
fn init_waits_for_an_external_holder_of_the_wiring_lock() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let h = harness("lockorder");
    let guard = h.manager.wiring_lock();

    let (tx, rx) = mpsc::channel();
    let manager = h.manager.clone();
    let handle = thread::spawn(move || {
        manager.init();
        tx.send(()).unwrap();
    });

    thread::sleep(Duration::from_millis(100));
    assert!(
        rx.try_recv().is_err(),
        "init() wired the chain while an external caller still held the lock"
    );

    drop(guard);
    rx.recv_timeout(Duration::from_secs(2))
        .expect("init() never proceeded after the lock was released");
    handle.join().unwrap();
}
