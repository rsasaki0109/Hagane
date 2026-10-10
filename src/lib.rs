//! Hagane: an independent, exact analytic B-rep kernel with explicitly limited operations.
mod booleans;
mod bored_prism_validation;
mod box_booleans;
mod circular_face_intersections;
mod circular_prism_validation;
mod circular_trims;
mod classification;
mod edge_simplify;
mod ellipse_planar;
mod extrusion_intersections;
mod face_intersections;
mod face_merge;
mod face_split;
mod geometry;
mod intersections;
mod math;
mod mesh;
mod mixed;
mod nurbs;
mod nurbs_refinement;
mod nurbs_surface;
mod nurbs_surface_curve_composition;
mod nurbs_tessellation_demo;
mod oblique_split;
mod operations;
mod planar;
mod predicates;
mod prism_validation;
mod sewing;
mod skew_boundary;
mod solid_split;
mod step;
mod step_read;
mod tilted_bore;
mod topology;
mod wasm;
mod workflow;
pub use booleans::*;
pub use bored_prism_validation::*;
pub use box_booleans::*;
pub use circular_face_intersections::*;
pub use circular_prism_validation::*;
pub use classification::*;
pub use edge_simplify::*;
pub use ellipse_planar::*;
pub use extrusion_intersections::*;
pub use face_intersections::*;
pub use face_merge::*;
pub use face_split::*;
pub use geometry::*;
pub use intersections::*;
pub use math::*;
pub use mesh::*;
pub use mixed::*;
pub use nurbs::*;
pub use nurbs_refinement::*;
pub use nurbs_surface::*;
pub use nurbs_tessellation_demo::*;
pub use oblique_split::*;
pub use operations::*;
pub use predicates::*;
pub use prism_validation::*;
pub use sewing::*;
pub use solid_split::*;
pub use step::*;
pub use step_read::*;
pub use tilted_bore::*;
pub use topology::*;
pub use workflow::*;
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    InvalidTopology(&'static str),
    Tessellation(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, detail) = match self {
            Self::InvalidInput(s) => ("invalid input", s),
            Self::Unsupported(s) => ("unsupported operation", s),
            Self::InvalidTopology(s) => ("invalid topology", s),
            Self::Tessellation(s) => ("tessellation failed", s),
        };
        write!(f, "{kind}: {detail}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

mod nurbs_surface_refinement;

mod nurbs_surface_boundary;
pub use nurbs_surface_boundary::*;

mod nurbs_face;
pub use nurbs_face::*;

mod nurbs_surface_tessellation;
pub use nurbs_surface_tessellation::*;

mod nurbs_surface_bounded_demo;
pub use nurbs_surface_bounded_demo::*;

mod nurbs_surface_extraction;
pub use nurbs_surface_extraction::*;

mod nurbs_surface_crease_demo;
pub use nurbs_surface_crease_demo::*;

mod nurbs_surface_trim;
mod nurbs_surface_trim_demo;
pub use nurbs_surface_trim_demo::*;

mod nurbs_holed_face;
pub use nurbs_holed_face::*;
mod nurbs_surface_hole_demo;
pub use nurbs_surface_hole_demo::*;

mod nurbs_parameter_curve;
mod nurbs_surface_edge;
pub use nurbs_surface_edge::*;
mod nurbs_surface_edge_demo;
pub use nurbs_surface_edge_demo::*;

mod nurbs_surface_wire;
pub use nurbs_surface_wire::*;

mod nurbs_polygon_face;
pub use nurbs_polygon_face::*;

mod nurbs_polygon_tessellation;
pub use nurbs_polygon_tessellation::*;

mod nurbs_polygon_demo;
pub use nurbs_polygon_demo::*;

mod nurbs_polygon_holed;
pub use nurbs_polygon_holed::*;

mod nurbs_graph_demo;
pub use nurbs_graph_demo::*;

mod nurbs_graph_solid;
pub use nurbs_graph_solid::*;

mod nurbs_graph_split;
pub use nurbs_graph_split::*;

mod nurbs_graph_hole_demo;
pub use nurbs_graph_hole_demo::*;

mod nurbs_graph_holed;
pub use nurbs_graph_holed::*;

mod nurbs_graph_classification_demo;
pub use nurbs_graph_classification_demo::*;

mod nurbs_graph_classification;

mod nurbs_graph_section_demo;
pub use nurbs_graph_section_demo::*;

mod nurbs_graph_vertical_section;
pub use nurbs_graph_vertical_section::*;

mod nurbs_graph_step_demo;
pub use nurbs_graph_step_demo::*;

mod nurbs_graph_step;

mod nurbs_graph_step_import_demo;
pub use nurbs_graph_step_import_demo::*;

mod nurbs_graph_step_import;
pub use nurbs_graph_step_import::*;

mod nurbs_graph_mass_properties;
pub use nurbs_graph_mass_properties::*;

mod nurbs_graph_inertia;
pub use nurbs_graph_inertia::*;

mod nurbs_graph_roof_section;
pub use nurbs_graph_roof_section::*;

mod nurbs_graph_polygon;
pub use nurbs_graph_polygon::*;

mod nurbs_graph_polygon_display;

mod nurbs_graph_polygon_mass;

mod nurbs_graph_polygon_split;
pub use nurbs_graph_polygon_split::*;

mod nurbs_graph_polygon_split_demo;
pub use nurbs_graph_polygon_split_demo::*;

mod nurbs_graph_polygon_material;

mod nurbs_graph_polygon_hole;
pub use nurbs_graph_polygon_hole::*;

mod nurbs_graph_polygon_hole_display;

mod nurbs_graph_polygon_classification;
mod nurbs_graph_polygon_classification_demo;
mod nurbs_graph_polygon_hole_mass;
pub use nurbs_graph_polygon_classification_demo::*;

mod nurbs_graph_polygon_step_demo;
pub use nurbs_graph_polygon_step_demo::*;

mod nurbs_graph_polygon_hole_demo;
pub use nurbs_graph_polygon_hole_demo::*;

mod nurbs_graph_polygon_demo;
pub use nurbs_graph_polygon_demo::*;

mod nurbs_graph_roof_section_demo;
pub use nurbs_graph_roof_section_demo::*;

mod nurbs_graph_polygon_inertia;

mod nurbs_graph_polygon_multi_hole;
pub use nurbs_graph_polygon_multi_hole::*;

mod nurbs_graph_polygon_multi_hole_display;
mod nurbs_graph_polygon_multi_hole_mass;

mod nurbs_graph_polygon_multi_hole_demo;
pub use nurbs_graph_polygon_multi_hole_demo::*;

mod nurbs_graph_polygon_multi_hole_step_demo;
pub use nurbs_graph_polygon_multi_hole_step_demo::*;

mod nurbs_graph_polygon_multi_hole_classification_demo;
pub use nurbs_graph_polygon_multi_hole_classification_demo::*;

mod nurbs_graph_polygon_step_import;
mod nurbs_graph_polygon_step_read;
pub use nurbs_graph_polygon_step_import::*;
mod nurbs_graph_polygon_step_import_demo;
pub use nurbs_graph_polygon_step_import_demo::*;
mod nurbs_graph_rational_roof_circle_demo;
pub use nurbs_graph_rational_roof_circle_demo::*;
mod nurbs_graph_circular_hole;
mod nurbs_graph_circular_hole_display;
mod nurbs_graph_circular_hole_mass;
pub use nurbs_graph_circular_hole::*;
mod nurbs_graph_circular_hole_demo;
pub use nurbs_graph_circular_hole_demo::*;
mod nurbs_graph_circular_hole_step_demo;
pub use nurbs_graph_circular_hole_step_demo::*;
mod nurbs_graph_circular_hole_classification;
mod nurbs_graph_circular_hole_classification_demo;
pub use nurbs_graph_circular_hole_classification_demo::*;
mod graph_workflow;
pub use graph_workflow::*;
mod graph_workflow_session;
pub use graph_workflow_session::*;
mod graph_workflow_demo;
pub use graph_workflow_demo::*;
mod edge_chamfer;
pub use edge_chamfer::*;
mod edge_chamfer_demo;
pub use edge_chamfer_demo::*;
mod edge_chamfer_multi;
pub use edge_chamfer_multi::*;
mod convex_contact_split;
mod edge_chamfer_multi_demo;
pub use edge_chamfer_multi_demo::*;
mod edge_chamfer_contact_demo;
pub use edge_chamfer_contact_demo::*;
mod edge_fillet;
pub use edge_fillet::*;
mod edge_fillet_demo;
pub use edge_fillet_demo::*;
mod step_bounded_import_demo;
pub use step_bounded_import_demo::*;
mod arc_line_prism_bore;
mod normal_prism_blind_bore;
pub use normal_prism_blind_bore::*;
mod normal_prism_convex_partition;
pub use normal_prism_convex_partition::*;
mod normal_prism_convex_demo;
pub use normal_prism_convex_demo::*;
mod normal_prism_blind_bores;
pub use normal_prism_blind_bores::*;
mod normal_prism_blind_bores_demo;
pub use normal_prism_blind_bores_demo::*;
mod normal_prism_blind_bore_demo;
mod normal_prism_blind_continue;
mod normal_prism_blind_validation;
pub use normal_prism_blind_continue::*;
mod normal_prism_blind_continue_demo;
pub use normal_prism_blind_bore_demo::*;
pub use normal_prism_blind_continue_demo::*;
mod arc_line_prism_validation;
pub use arc_line_prism_bore::*;
mod arc_line_prism_bore_demo;
pub use arc_line_prism_bore_demo::*;

mod arc_line_prism_split;
pub use arc_line_prism_split::*;
mod arc_line_prism_split_demo;
pub use arc_line_prism_split_demo::*;

mod arc_line_prism_split_components_demo;
pub use arc_line_prism_split_components_demo::*;

mod normal_prism_arc_line_boolean;
pub use normal_prism_arc_line_boolean::*;
mod normal_prism_arc_boolean_demo;
pub use normal_prism_arc_boolean_demo::*;

mod normal_prism_region_boolean;
pub use normal_prism_region_boolean::*;
mod normal_prism_region_boolean_demo;
pub use normal_prism_region_boolean_demo::*;

mod prism_workflow;
pub use prism_workflow::*;

mod prism_workflow_demo;
pub use prism_workflow_demo::*;

mod nurbs_frustum_step_import;
mod nurbs_frustum_step_read;
pub use nurbs_frustum_step_import::*;
mod nurbs_frustum_cardinal_step_import_demo;
pub use nurbs_frustum_cardinal_step_import_demo::*;
mod nurbs_frustum_translated_step_import_demo;
pub use nurbs_frustum_translated_step_import_demo::*;
mod nurbs_frustum_step_import_demo;
pub use nurbs_frustum_step_import_demo::*;
mod nurbs_frustum;
pub use nurbs_frustum::*;
mod nurbs_frustum_classification;
mod nurbs_frustum_split;
pub use nurbs_frustum_split::*;
mod nurbs_frustum_partitions_demo;
pub use nurbs_frustum_partitions_demo::*;
mod nurbs_frustum_split_demo;
pub use nurbs_frustum_split_demo::*;
mod nurbs_frustum_demo;
mod nurbs_frustum_display;
mod nurbs_frustum_mass;
pub use nurbs_frustum_demo::*;

mod nurbs_frustum_workflow_demo;
pub use nurbs_frustum_workflow_demo::*;
mod nurbs_frustum_workflow;
pub use nurbs_frustum_workflow::*;

mod nurbs_frustum_intersection;
pub use nurbs_frustum_intersection::*;
mod nurbs_frustum_line_demo;
pub use nurbs_frustum_line_demo::*;
mod nurbs_frustum_segment_demo;
pub use nurbs_frustum_segment_demo::*;

mod nurbs_frustum_line_intersection;
pub use nurbs_frustum_line_intersection::*;

mod nurbs_frustum_plane_section_demo;
pub use nurbs_frustum_plane_section_demo::*;

mod nurbs_frustum_oblique_display;

mod nurbs_frustum_plane_split_demo;
pub use nurbs_frustum_plane_split_demo::*;

mod nurbs_frustum_plane_section;
pub use nurbs_frustum_plane_section::*;

mod nurbs_frustum_oblique_split;
pub use nurbs_frustum_oblique_split::*;
