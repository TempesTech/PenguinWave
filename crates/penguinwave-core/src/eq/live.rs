//! Live coefficient updates on the running filter chain.

use crate::eq::biquad::Biquad;
use penguinwave_pipewire::PipeWireBackend;
use penguinwave_proto::PwError;

/// Coefficient controls for one graph node, as (control, value) pairs.
pub fn coefficient_params(graph_node: &str, bq: &Biquad) -> Vec<(String, f32)> {
    [
        ("b0", bq.b0),
        ("b1", bq.b1),
        ("b2", bq.b2),
        ("a1", bq.a1),
        ("a2", bq.a2),
    ]
    .iter()
    .map(|(port, value)| (format!("{graph_node}:{port}"), *value))
    .collect()
}

/// Push a batch of control updates to a filter-chain node in one call.
pub fn push(
    backend: &dyn PipeWireBackend,
    node_name: &str,
    params: &[(String, f32)],
) -> Result<(), PwError> {
    if params.is_empty() {
        return Ok(());
    }
    let node_id = backend.resolve_node_id(node_name)?;
    backend.set_node_props(node_id, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eq::biquad::IDENTITY;

    #[test]
    fn coefficient_params_shape() {
        let params = coefficient_params("eq_b3", &IDENTITY);
        assert_eq!(params.len(), 5);
        assert_eq!(params[0], ("eq_b3:b0".to_string(), 1.0));
        assert_eq!(params[3], ("eq_b3:a1".to_string(), 0.0));
    }
}
