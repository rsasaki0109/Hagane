# Tensor-product NURBS surfaces

`NurbsSurface` is an immutable standalone geometry object for an **untrimmed,
clamped, nonperiodic, positive-weight rational B-spline surface**. It evaluates
points, analytic first partials, and oriented normals. [Surface refinement and
sections](nurbs-surface-refinement.md) adds shape-preserving knot insertion, exact
isocurves and oriented rectangular boundary data. It does not create a
shell or solid, and provides no surface intersections or Boolean operations.
The separate [NurbsFace](nurbs-face.md) API now retains a complete rectangular
patch as an open face using existing B-rep topology; arbitrary trims and closed
NURBS solid operations remain unsupported.

## Control grid and checked input

```rust
use hagane::{NurbsSurface, Point3};
let knots = vec![0.0, 0.0, 1.0, 1.0];
let plane = NurbsSurface::new(
    [1, 1], [knots.clone(), knots], [2, 2],
    vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0),
         Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    vec![1.0; 4],
)?;
let point = plane.evaluate(0.5, 0.5)?;
let evaluation = plane.partials(0.5, 0.5)?;
let normal = evaluation.normal()?;
```

Constructor arguments are `[degree_u, degree_v]`, the two knot vectors,
`[count_u, count_v]`, flattened control points and corresponding weights.
**U-major ordering** means point `(i,j)` is at index `i * count_v + j`.
Each degree is 1..16; total control count is at most 65,536. Both axes use the
same knot validation and inward/one-sided rules as [NURBS curves](nurbs.md).
The grid product is checked for overflow and must exactly match the point and
weight cardinalities. Empty/ragged grids, invalid knots, nonfinite coordinates,
and zero/negative weights fail construction. Weight normalization and numerical
range guards are shared with curves, so the two evaluators cannot drift in
these input contracts. Accessors provide read-only slices/counts/degrees.

The domains are closed intervals with no extrapolation or fuzzy snapping.
First partials are in length per U/V parameter unit, which need not be `[0,1]`.
A degenerate control net can still define points. Construction does not certify
regularity or global non-self-intersection; normals are checked at evaluation.

## Tensor evaluation and derivatives

With `H_ij = (w_ij P_ij, w_ij)`, the homogeneous surface is

`H(u,v) = Σ_i Σ_j N_i,p(u) M_j,q(v) H_ij`.

The point is `P = H_xyz / H_w`. The implementation applies local homogeneous
de Boor evaluation in V then U; no intermediate projection discards weights.
V derivatives use each row's differentiated spline and are then combined in U.
U derivatives differentiate the combined U spline. Rational first partials use

`P_u = (H_u,xyz - P H_u,w) / H_w`

and the corresponding V expression. Production derivatives are analytic;
finite differences are a test cross-check only. A quadratic rational quarter
circle extruded along the second parameter represents an exact quarter-cylinder
surface, checked against its radius, height and radial normal.

`partials(u,v)` and `normal(u,v)` reject potentially C0 interior knots in either
axis. `evaluate_with_partials(u,v,[KnotSide::Left, KnotSide::Right])` requests
explicit limits per axis; endpoints always use inward limits. Position remains
continuous by the knot multiplicity contract, but derivatives/normals may differ.

## Normal contract

`SurfaceEvaluation` contains `point`, `du` and `dv`. Its `normal()` is the unit
vector parallel to `du × dv`. Tangents are scaled before the cross product,
avoiding overflow/underflow for regular but tiny/large tangent magnitudes.
Zero/nonfinite tangents and angles with sine at most `64 × f64::EPSILON` are
explicit errors. A singularity never becomes an invented axis or zero normal.
This is a numerical angular guard, not a general surface-regularity certificate.

## Display sampling and demo

The browser now uses [bounded multi-span surface triangles](nurbs-surface-tessellation.md)
through the retained `NurbsFace`. `tessellate_bounded(error,max_cells)` returns
shared vertices, original UV cell ranges and per-cell Bernstein bounds.
[Bezier extraction](nurbs-surface-extraction.md) supports exact patches including
C0, with [one-sided display normals](nurbs-surface-crease.md) at creases. The uniform API below
is retained for compatibility.


`sample_grid([cells_u,cells_v])` creates a display `Mesh` with 1..256 uniform
parameter cells per axis. It is an open patch, not a closed CAD solid. All
`face_ids` are 0, denoting the display patch rather than a topological face.
Positions and normals are evaluated from the exact surface; triangles are
checked for finite, nondegenerate geometry and consistency with sampled normals.
A grid that is too coarse for the sampled orientation returns `Tessellation`.
This compatibility method still rejects C0 interior knot lines; use the
bounded tessellator for explicit [one-sided crease normals](nurbs-surface-crease.md).

**There is no certified chord-error, global injectivity or surface-coverage
claim for this sampling method.** It does not implement trimmed face meshing.
The existing analytic B-rep tessellator's circle sagitta bound remains separate.

```sh
cargo run --locked --example nurbs_surface_bounded
cargo run --locked --example nurbs_surface_bounded -- 35 2 0.3 0.7 0.05
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Choose the **Surfaces** link in the browser. The demo is a degree-2-by-2 patch
with a 3-by-3 control net and variable center height/weight. A shape-preserving
4-by-4 refinement supplies bounded display curves for four boundaries and two
selected U/V sections, with same-parameter affine UV maps. U/V controls select
a point and its analytic normal (cyan marker). Drag/keyboard orbit, wheel zoom,
bounded surface-error and rotation controls use the same display-only WebGL viewer as
the solid demo. The open patch is drawn from either side; back-side lighting
flips the shading normal only, not the kernel's oriented normal.

The browser export
`hagane_generate_surface_bounded(height,weight,u,v,chord_error)` uses the shared
WASM JSON output buffer and returns status 0/1. The height fixture is limited
to `[-100,100]`; core surfaces do not have that arbitrary demo limit. Any
`hagane_generate*` call invalidates previously returned bytes. Native/WASM tests
retain compatibility grid fixtures and additionally compare full bounded meshes,
cell ranges/bounds, selected points and section curves; browser tests verify
parameter changes, geometry changes, error controls and bound compliance.

![Actual Rust/WASM bounded NURBS face](nurbs-surface-bounded.png)

## Remaining work

Periodic axes, higher/mixed derivatives, bounded arbitrary trimmed surface display
meshing, arbitrary trimming, intersections, cross-face sewing and closed
NURBS solids remain unimplemented. Rectangular open-face topology is supported
by [NurbsFace](nurbs-face.md). Surface evaluation alone does
not imply support for any of those operations.
