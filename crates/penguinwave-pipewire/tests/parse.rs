//! Parser tests against fixtures captured from a live session.

use penguinwave_pipewire::parse::*;
use penguinwave_proto::PortDirection;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("fixtures/", $name))
    };
}

#[test]
fn sinks_cover_every_device_in_the_session() {
    let sinks = parse_sinks(fixture!("sinks_full.txt"));
    assert_eq!(sinks.len(), 6, "{sinks:#?}");

    let names: Vec<_> = sinks.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"game_sink"));
    assert!(names.contains(&"chat_sink"));
    assert!(names.iter().any(|n| n.starts_with("bluez_output.")));
    assert!(names.iter().any(|n| n.contains("hdmi")));
}

/// Managed sinks are identified by the factory that creates them, not by a
/// blocklist of known hardware prefixes: the old `grep -v alsa_output|bluez...`
/// approach misclassifies any sink whose name it has not seen before.
#[test]
fn managed_sinks_are_detected_by_factory() {
    let sinks = parse_sinks(fixture!("sinks_full.txt"));
    let managed: Vec<_> = sinks
        .iter()
        .filter(|s| s.managed)
        .map(|s| &s.name)
        .collect();
    assert_eq!(managed, vec!["game_sink", "chat_sink"]);
}

#[test]
fn output_devices_exclude_our_own_sinks() {
    let devices = parse_output_devices(fixture!("sinks_full.txt"));
    assert_eq!(devices.len(), 4);
    assert!(!devices.iter().any(|d| d.name.ends_with("_sink")));
}

/// The fixture was captured on a comma-decimal locale, where dB reads
/// `-18,06`. Only the percentage is parsed, and the padding around it varies.
#[test]
fn volume_survives_locale_and_padding() {
    assert_eq!(
        parse_volume_pct("front-left: 32768 /  50% / -18,06 dB,   front-right: 32768 /  50%"),
        Some(50)
    );
    assert_eq!(
        parse_volume_pct("front-left: 65536 / 100% / 0,00 dB"),
        Some(100)
    );
    assert_eq!(parse_volume_pct("front-left: 0 /   0% / -inf dB"), Some(0));
    assert_eq!(parse_volume_pct("balance 0,00"), None);
    // Boosted above 100% clamps rather than overflowing a u8.
    assert_eq!(
        parse_volume_pct("front-left: 98304 / 150% / 7,00 dB"),
        Some(100)
    );
}

#[test]
fn sink_volume_and_mute_are_read() {
    let sinks = parse_sinks(fixture!("sinks_full.txt"));
    let game = sinks.iter().find(|s| s.name == "game_sink").unwrap();
    assert_eq!(game.volume, 50);
    assert!(!game.is_muted);
    assert_eq!(game.description, "Game");
}

#[test]
fn streams_carry_identity_for_revalidation() {
    let streams = parse_sink_inputs(fixture!("sink_inputs_full.txt"));
    assert_eq!(streams.len(), 5, "{streams:#?}");

    let game = streams
        .iter()
        .find(|s| s.stream.app_name == "Marvel's Spider-Man 2")
        .expect("game stream");
    assert_eq!(game.stream.pid, Some(284433));
    assert_eq!(game.sink_id, 1944);
    assert_eq!(game.volume, 100);
    assert!(!game.is_muted);
}

/// Streams with no `application.name` (PipeWire's own nodes) must still parse
/// rather than being dropped, or they vanish from routing entirely.
#[test]
fn streams_without_an_application_name_survive() {
    let streams = parse_sink_inputs(fixture!("sink_inputs_full.txt"));
    let anonymous: Vec<_> = streams
        .iter()
        .filter(|s| s.stream.app_name.is_empty())
        .collect();
    assert_eq!(anonymous.len(), 2, "{anonymous:#?}");
    assert!(anonymous.iter().all(|s| s.stream.index != 0));
}

/// One process can own several concurrent streams, so `app_name` + `pid` do not
/// identify a stream — only the recycled index separates them.
///
/// Revalidation is therefore scoped to what it can actually prove: that an
/// index still belongs to the same application. It cannot tell one of an app's
/// streams from another, and must not claim to.
#[test]
fn identity_tuple_does_not_disambiguate_streams_of_one_process() {
    let streams = parse_sink_inputs(fixture!("sink_inputs_full.txt"));

    let mut by_identity: std::collections::HashMap<(String, Option<u32>), Vec<u32>> =
        Default::default();
    for s in &streams {
        by_identity
            .entry((s.stream.app_name.clone(), s.stream.pid))
            .or_default()
            .push(s.stream.index);
    }

    let shared = by_identity
        .values()
        .find(|indices| indices.len() > 1)
        .expect("fixture should contain one process with several streams");
    let unique: std::collections::HashSet<_> = shared.iter().collect();
    assert_eq!(
        unique.len(),
        shared.len(),
        "indices must still differ: {shared:?}"
    );
}

#[test]
fn short_listings_parse() {
    let sinks = parse_sinks_short(fixture!("sinks_short.txt"));
    assert_eq!(sinks.len(), 6);
    assert!(sinks.contains(&(63, "game_sink".to_string())));
}

#[test]
fn empty_listing_is_not_an_error() {
    assert!(parse_sinks_short(fixture!("sinks_short_empty.txt")).is_empty());
    assert!(parse_sinks("").is_empty());
    assert!(parse_sink_inputs("").is_empty());
    assert!(parse_links("").is_empty());
}

#[test]
fn links_are_reported_once_from_the_source_side() {
    let links = parse_links(fixture!("pw_link_ids.txt"));
    assert!(!links.is_empty());

    // Collecting both |-> and |<- would report every link from both ends.
    let mut seen = std::collections::HashSet::new();
    for l in &links {
        assert!(
            seen.insert((l.source.id, l.target.id)),
            "link {l:?} reported twice"
        );
        assert!(!l.source.node_name.is_empty());
        assert!(!l.target.port_name.is_empty());
        assert!(l.source.id.is_some(), "port id required: {l:?}");
        assert!(l.target.id.is_some(), "port id required: {l:?}");
    }
}

/// Several nodes of one application share a name, so `node:port` matches more
/// than one port. Anything that unlinks by name alone tears down the wrong
/// link; ids are what disambiguate.
#[test]
fn node_names_are_not_unique() {
    let links = parse_links(fixture!("pw_link_ids.txt"));

    // A port may appear in several links (fan-out), so count distinct ids per
    // name rather than occurrences.
    let mut ids_by_name: std::collections::HashMap<(&str, &str), std::collections::HashSet<u32>> =
        Default::default();
    for l in links.iter().flat_map(|l| [&l.source, &l.target]) {
        if let Some(id) = l.id {
            ids_by_name
                .entry((l.node_name.as_str(), l.port_name.as_str()))
                .or_default()
                .insert(id);
        }
    }

    let ambiguous: Vec<_> = ids_by_name
        .iter()
        .filter(|(_, ids)| ids.len() > 1)
        .collect();
    assert!(
        !ambiguous.is_empty(),
        "fixture should contain a name shared by several distinct ports"
    );
}

#[test]
fn ports_come_from_pw_dump() {
    let ports = parse_ports_from_dump(fixture!("pw_dump_all.json")).unwrap();
    let game: Vec<_> = ports
        .iter()
        .filter(|p| p.node_name == "game_sink")
        .collect();
    assert_eq!(game.len(), 4, "{game:#?}");

    let playback: Vec<_> = game
        .iter()
        .filter(|p| p.direction == PortDirection::Input)
        .map(|p| p.port_name.as_str())
        .collect();
    assert_eq!(playback, vec!["playback_FL", "playback_FR"]);

    let monitor: Vec<_> = game
        .iter()
        .filter(|p| p.direction == PortDirection::Output)
        .map(|p| p.port_name.as_str())
        .collect();
    assert_eq!(monitor, vec!["monitor_FL", "monitor_FR"]);
}

/// `pw-cli` exits 0 on an unknown command, so a zero status is not proof of
/// success. The pre-split code parsed this error text as port data and
/// reported "no ports" instead of a failure.
#[test]
fn tool_error_is_detected_despite_zero_exit() {
    let output = fixture!("pw_cli_unknown_cmd.txt");
    assert!(is_tool_error(output), "{output:?}");
    assert!(!is_tool_error(fixture!("pw_link_list.txt")));
}

#[test]
fn pactl_failure_text_is_recognised() {
    let output = fixture!("pactl_error_missing_sink.txt");
    assert!(output.contains("No such entity"), "{output:?}");
}

/// Properties are indented deeper than fields; a parser that misreads the
/// boundary silently drops `factory.name` and every sink stops being managed.
#[test]
fn fields_and_properties_do_not_bleed() {
    let blocks = parse_blocks(fixture!("sinks_full.txt"), "Sink #");
    let game = blocks.iter().find(|b| b.id == 63).unwrap();
    assert_eq!(game.field("Name"), Some("game_sink"));
    assert_eq!(game.prop("factory.name"), Some("support.null-audio-sink"));
    assert!(!game.fields.contains_key("node.name"));
    assert!(!game.props.contains_key("Name"));
}
