//! Hagane: an independent, exact analytic B-rep kernel with explicitly limited operations.
mod face_intersections;
mod geometry;
mod intersections;
mod math;
mod mesh;
mod mixed;
mod nurbs;
mod nurbs_surface;
mod operations;
mod planar;
mod predicates;
mod topology;
mod wasm;
pub use face_intersections::*;
pub use geometry::*;
pub use intersections::*;
pub use math::*;
pub use mesh::*;
pub use mixed::*;
pub use nurbs::*;
pub use nurbs_surface::*;
pub use operations::*;
pub use predicates::*;
pub use topology::*;
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
