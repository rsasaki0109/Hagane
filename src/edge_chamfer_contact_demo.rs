//! Explicit contact-capable planar edge chamfer demonstration.
use crate::*;
pub fn edge_chamfer_contact_demo_json(values: &[f64]) -> Result<String> {
    crate::edge_chamfer_multi_demo::edge_chamfer_mode_demo_json(values, true)
}
