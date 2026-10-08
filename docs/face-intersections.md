# Transverse planar face clipping

Hagane now clips an infinite line against the exact trims of a validated
planar B-rep face, and intersects two planar faces by clipping their shared
supporting-plane line against both trim regions. This is intersection
infrastructure: it does not split faces, modify topology, sew shells or perform
a general Boolean.

## Line clipping

```rust
use hagane::{Point3, Vec3, GeometryTolerance, clip_line_to_planar_face};
// `solid` is an existing valid B-rep with a planar face at index 0.
let clipped = clip_line_to_planar_face(
    &solid, 0, Point3::new(-10.0, 0.5, 0.0),
    Vec3::new(2.0, 0.0, 0.0), GeometryTolerance::default(),
)?;
for interval in clipped.intervals {
    let [start, end] = interval.parameter_range;
    // Line parameter follows the original input direction, not a unit direction.
    // Endpoints identify the original wire/coedge/edge and owning edge parameter.
    println!("{start} .. {end}: edge {} to {}",
        interval.start.edge, interval.end.edge);
}
```

The entire owning `Solid` is validated before topology is read. Supported
planar trims are the existing simple polygon, disk, circular-hole and mixed
line/arc regions, including concavity, multiple disjoint holes and either face
orientation. A full circle's periodic seam can be crossed; bounded trim
vertices cannot be crossed in this initial transverse-only API.

`PlanarLineClip.events` lists all proper boundary crossings sorted by the
caller's original line parameter. Each `PlanarBoundaryEvent` records the 3D
point, line parameter, wire index, coedge index, shared edge index and edge
parameter. The edge parameter follows the stored 3D curve/pcurve, independently
of coedge direction. `intervals` contains the material interior spans, with
boundary endpoints included. Disjoint cuts return empty events/intervals.

The algorithm solves line/line and line/circle equations in plane UV space,
filters circular roots to their analytic arc spans, and checks root agreement
with the 3D line, edge and surface. It sorts boundary events and classifies
midpoints analytically against outer and hole rings. No display polygon or mesh
is used. Direction normalization and conversion preserve very small/large
finite original direction components; unrepresentable parameters fail.

## Two planar faces

```rust
use hagane::{GeometryTolerance, intersect_planar_faces, PlanarFacesIntersection};
let result = intersect_planar_faces(
    &first_solid, first_face_index, &second_solid, second_face_index,
    GeometryTolerance::default(),
)?;
if let PlanarFacesIntersection::Segments(segments) = result {
    for segment in segments {
        let midpoint = segment.curve.evaluate(0.5);
        let first_uv = segment.first.evaluate(0.5);
        let second_uv = segment.second.evaluate(0.5);
    }
}
```

Both solids are validated. The result is `Parallel` for supporting planes
classified as parallel under the angular policy, or `Segments`. Empty segments
mean the nonparallel trimmed faces are disjoint. Coplanar overlays return
`Unsupported`; coincident supporting planes do not establish coincident faces.

Each `PlanarFaceIntersectionSegment` contains a finite `Curve::Line` and affine
pcurves on both input faces. **All three use the same normalized [0,1]
parameter.** Its separate `parameter_range` records the interval on the
unit-speed infinite supporting-plane intersection line. Geometry checks verify
pcurve agreement at both endpoints and the midpoint. Direction follows the
ordered supporting-plane normals, independently of face orientation. Holes and
concavity can produce multiple separate intersection edges. Reversing input
faces reverses the supporting intersection direction.

For boundary provenance, use `clip_line_to_planar_face` on each face with the
line returned by `intersect_plane_plane`. The finite-face convenience result
retains shared-line ranges and pcurves; it does not create new topological edges
or update the original coedges.

## Explicit domain and numerical limits

- Only **proper transverse cuts** are supported. Tangency/near-tangency,
  vertex/near-vertex passage, boundary overlap and unresolved near-parallel
  crossings return explicit `Unsupported` errors. Contact-only face overlap
  and near-zero shared intervals are also rejected.
- Supporting-plane membership uses a numerical allowance bounded by the model
  length budget. A materially offset parallel line or a direction with normal
  component beyond `64 * epsilon` is invalid; a merely policy-coincident line
  is not accepted as an exact planar cut.
- Clearance uses the structured length budget at local trim feature scale
  (maximum line length/circle radius). Event spacing additionally accounts for
  `64 * epsilon * abs(line_travel)` to reject unresolvable intervals.
- Midpoints within the boundary tolerance, merged events, odd crossing counts,
  unrepresentable parameters and lost 3D edge/line/surface agreement fail
  explicitly. Small features require a smaller configured linear tolerance.
- Circular roots/span tests and metric checks remain checked f64 computations,
  with a 1e-10 angular span allowance, not certified exact circular/3D predicates.
  UV classification uses the existing analytic ray/circle implementation.
- Cylindrical faces, arbitrary surface trims, coplanar overlays, contact graphs,
  vertex/tangent handling and sewing remain future work.
  [Scoped line/arc face subdivision](face-split.md) uses this clipping API.

## Runnable fixture and verification

```sh
cargo run --locked --example face_clipping          # rotated cap/hole + face pair
cargo run --locked --example face_clipping -- 0.5 0 # unrotated analytic fixture
./scripts/build-web.sh
node scripts/test-wasm.mjs
```

The JSON fixture clips a box's circular-bore cap horizontally and intersects
that cap with a vertical box face. Offset changes both cuts; placement rotates
all input B-reps together. For offset 0.5 and unrotated placement, horizontal
intervals on the original direction `(2,0,0)` are
`[3, (10-sqrt(0.75))/2]` and `[(10+sqrt(0.75))/2, 7]`.
The two trimmed-face segments have total length `4 - 2*sqrt(0.75)`.
The unchanged B-rep volume is `96 - 2*pi`.

`hagane_generate_face_clipping(offset, placement_angle)` exports the same
fixture in WASM under the existing output-buffer/error contract. Native tests
cover polygon/concave/circular/arc trims, annuli, multiple holes, clockwise
notches, reversed direction, rigid placement, boundary provenance, shared
normalized pcurves, parallel/disjoint/coplanar/contact faces, small dimensions,
unrepresentable parameters and rejected invalid/unsupported cuts. Native/WASM
runtime cases independently check intervals, segment lengths, pcurves, event
ordering, unchanged analytic volume, errors and recovery.
