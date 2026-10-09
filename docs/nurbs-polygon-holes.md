# Rectangular inner wires on convex rational faces

![Actual Rust/WASM holed rational polygon face](nurbs-polygon.png)

`NurbsPolygonHoledFace` retains a counterclockwise convex UV outer boundary and
up to 16 separated rectangular UV inner wires. Each inner wire is clockwise,
with exact rational surface curves, affine same-parameter pcurves and shared
corner vertex indices. Public vertices/edges and face wire references use one
common index space. The supporting NURBS surface is retained unchanged. This is
an open B-rep face, not a closed solid or a mesh Boolean.

```rust
use hagane::{NurbsPolygonHoledFace, Tolerance};
fn example(surface: hagane::NurbsSurface) -> hagane::Result<()> {
let face = NurbsPolygonHoledFace::new(
    surface,
    vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
    vec![[[0.2, 0.3], [0.2, 0.3]]], // [U range, V range]
    1,
    Tolerance::default(),
)?;
let _display = face.tessellate_bounded(0.1, 65536, Tolerance::default())?;
Ok(())
}
```

Outer corners are supplied once; closure is implicit. Hole ranges must be finite,
ordered, resolved and strictly inside the outer convex polygon. Overlap, contact,
nearly touching inner/outer boundaries, reversed ranges and excessive counts are
rejected. UV clearance uses source-domain arithmetic guards, independently of
physical-length tolerance. It does not certify 3D separation or surface injectivity
on a folded supporting surface. Validation checks the supporting source, entity
counts, shared indices, outer/inner traversal and canonical geometry, including
publicly corrupted entities.

Display partitions the original convex polygon into convex cells at hole range
coordinates and source C0 knot lines before triangulation. Intersections on shared
cell edges reuse node identities. Whole cells inside openings are removed; retained
cells are fan-triangulated and uniformly subdivided. No display triangle crosses
an opening or a C0 crease. Normals are evaluated only on retained material; a
singularity at a removed source point cannot fail display through normal sampling.
At creases, shared geometry and explicit one-sided normal variants remain.

Existing rational Bernstein derivative envelopes and barycentric Taylor bounds
apply to retained triangles. Source extraction and derivative envelopes still use
the whole surface, so conditioning or curvature in removed regions can trigger
conservative rejection. There is no formal interval/global regularity certificate.
Display permits 1–65536 triangles and ten subdivision levels. Intermediate convex
cell triangulation counts also consume the triangle budget, and clipping visits
at most 16 million cell vertices. Unresolved intersections, collapsed/reversed UV
triangles, singular retained normals, nonfinite bounds and exhausted budgets return
explicit errors. No fuzzy snapping or partial-success meshes are returned.

```sh
cargo run --example nurbs_polygon_holes > two-openings.obj
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

`web/surface-polygon.html` exposes one editable rectangular opening on a triangular
outer boundary in single, C1 and C0 modes; native construction supports 3–64 convex
outer corners and up to 16 holes. Invalid edits preserve the accepted display.
The shared diagnostic example and WASM ABI accept height, weight, error, mode,
six outer corner coordinates, then hole U min/max and V min/max:

```sh
cargo run --example nurbs_polygon_hole -- 35 1 0.5 0 0.125 0.125 0.875 0.25 0.25 0.875 0.375 0.5 0.375 0.5
```

Native independent tests verify rational positions/bounds, exact inner winding,
material UV area, all outer/inner boundary halfedges, shared geometry, C0 crossings
and aligned hole sides, multiple decimal holes, 16-hole resource limits, excluded
singularity sampling and invalid/corrupted inputs. Native/WASM comparisons check
complete JSON and independent curve/surface formulas, material exclusion, area,
B-rep pcurves and recovery. Browser checks exercise editing, rejection, recovery,
actual rendering and captured output.

Nonrectangular holes, concave/general outer trim loops, sewing, closed rational
solids, general curved Booleans and NURBS STEP remain future work. No dependencies
or OCCT source were added; original code is MIT OR Apache-2.0.
