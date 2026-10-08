# Planar solid point classification

![Actual WASM point classification demo](classification.png)

`classify_point_in_solid(&Solid, Point3, GeometryTolerance)` returns
`PointLocation::{Inside, Outside, Boundary}` for a validated closed oriented
B-rep with planar faces and straight boundary edges. Polygon holes, concavity,
face subdivisions, rigid placement and skew polygon extrusion are supported.
Curved surfaces or boundaries return `Unsupported`, even for a query outside
the bounds. Invalid topology and nonfinite coordinates return errors.

```rust
use hagane::*;
let tolerance = GeometryTolerance::default();
let solid = make_box(BoxSpec {
    min: Point3::new(0.0, 0.0, 0.0),
    size: Vec3::new(4.0, 4.0, 4.0),
}, tolerance.absolute())?;
assert_eq!(classify_point_in_solid(&solid,
    Point3::new(2.0, 2.0, 2.0), tolerance)?, PointLocation::Inside);
assert_eq!(classify_point_in_solid(&solid,
    Point3::new(2.0, 2.0, 0.0), tolerance)?, PointLocation::Boundary);
```

## Boundary and ray contract

The shape is validated before any bounds shortcut or query result. Its local
bounds diagonal defines the length budget `max(linear, relative * diagonal)`;
world offsets and query distance do not enlarge it. A point within that budget
of a trimmed face is `Boundary`. Distance combines plane-normal distance and
projected trim distance using `hypot`. A projection inside the material region
has zero lateral distance; projections in holes or outside the outer loop use
the nearest exact polygon segment. This is a Euclidean band, including edges
and vertices, rather than independent coordinate-wise snapping.

For remaining points, ray/plane hits are evaluated against exact polygon trims
using filtered exact 2D predicates. Near-parallel candidates, vertex/edge hits,
hits within the boundary budget, and unresolved reconstruction discard that
ray. Sorted material crossings must alternate entry/exit according to face
orientation, end outside at infinity, and have resolvable separation. Parity
then determines membership. Two independent successful directions must agree;
inconsistent orientation, disagreement or exhaustion of twelve deterministic
candidate directions returns `Unsupported` rather than guessing a result.
A very large angular tolerance can exhaust the candidate set.

No display mesh is consulted. Numerical plane intersections and metric distances
use checked f64 calculations; 3D decisions are not certified exact predicates.
The input must be a geometrically non-self-intersecting solid within the existing
supported trim domain. Existing structural validation checks topology, trims,
pcurves, connectivity and orientation; it is not a general geometric
self-intersection detector. Curved/NURBS classification, coplanar arrangements
and Boolean selection remain subsequent work.

## Native/WASM/browser evidence

Run `cargo run --locked --example classification -- -10 0 0` for a point query
against the fixture, or add `--mesh` for its B-rep-derived display JSON.
The **Classify** link opens `classification.html`: a concave L-shaped extrusion
with a polygon through-hole, volume 69336 mm³. X/Y/Z controls and fixed probes
query material, through-hole, notch, top face, corner and external points. The
cyan stem marks the query location; interior points can be occluded by the solid.
The same Rust fixture and classifier execute natively and in WASM.

Tests cover an independent analytic box grid, concave/hollow trims, face/edge/
vertex and near-boundary points, Euclidean corner distance, relative tolerance,
ray vertex degeneracy, face subdivision, rigid/skew placement, tiny models,
exhausted candidates, curved-input rejection, invalid topology and nonfinite
queries. WASM tests compare native results, boundary bands, recovery and the
actual display mesh. Browser tests exercise all probes, coordinate changes,
orbit and responsive layout.
