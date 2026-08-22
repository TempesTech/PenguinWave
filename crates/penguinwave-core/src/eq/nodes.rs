//! PipeWire node names for the EQ filter chains.

use penguinwave_proto::{EqChainId, CHAT_SINK, GAME_SINK};

/// Capture-side node: carries the DSP controls, captures the sink's monitor.
pub fn sink_node_name(chain: EqChainId) -> &'static str {
    match chain {
        EqChainId::Game => "penguinwave_eq_game",
        EqChainId::Chat => "penguinwave_eq_chat",
    }
}

/// Playback-side node: feeds the output device.
pub fn output_node_name(chain: EqChainId) -> &'static str {
    match chain {
        EqChainId::Game => "penguinwave_eq_game_out",
        EqChainId::Chat => "penguinwave_eq_chat_out",
    }
}

/// Virtual sink whose monitor the chain captures.
pub fn source_sink_for(chain: EqChainId) -> &'static str {
    match chain {
        EqChainId::Game => GAME_SINK,
        EqChainId::Chat => CHAT_SINK,
    }
}

pub fn chain_for_sink(sink_name: &str) -> Option<EqChainId> {
    match sink_name {
        GAME_SINK => Some(EqChainId::Game),
        CHAT_SINK => Some(EqChainId::Chat),
        _ => None,
    }
}

/// A node Penguin Wave manages itself — never a valid output target.
///
/// Routing an EQ output into one creates a feedback loop, and the user's
/// default sink is often `game_sink`.
pub fn is_managed_node(node: &str) -> bool {
    node == GAME_SINK
        || node == CHAT_SINK
        || EqChainId::ALL
            .iter()
            .any(|&c| node == sink_node_name(c) || node == output_node_name(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_nodes_are_never_output_targets() {
        assert!(is_managed_node(GAME_SINK));
        assert!(is_managed_node(CHAT_SINK));
        assert!(is_managed_node("penguinwave_eq_game"));
        assert!(is_managed_node("penguinwave_eq_chat_out"));
        assert!(!is_managed_node(
            "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo"
        ));
    }

    #[test]
    fn every_chain_maps_back_from_its_sink() {
        for c in EqChainId::ALL {
            assert_eq!(chain_for_sink(source_sink_for(c)), Some(c));
        }
        assert_eq!(chain_for_sink("alsa_output.x"), None);
    }
}
