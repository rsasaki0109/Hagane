# Crease-aware bounded rational polygon display

`NurbsPolygonFace::tessellate_crease_bounded` displays convex straight-UV trim
boundaries across C0 as well as C1 source knots. The supporting NURBS surface,
exact lifted boundary curves and same-parameter pcurves remain the CAD geometry.
No mesh Boolean or fitted surface is used. The older `tessellate_bounded` and
bilinear entry point retain their structural-C1 requirement.

The initial convex fan is split at every structural C0 source knot. Half-plane
clipping snaps the crossed axis to the original knot; each intersection is keyed
by the shared edge's node indices and reused by both incident triangles. Uniform
midpoint subdivision then retains shared indices. No emitted triangle crosses a
C0 line. The existing per-span Bernstein Hessian envelopes bound the Taylor
remainder on either side, including C1 lines within a side. Bounds include the
existing engineering parameter/world arithmetic reserves. Intersection and UV
arithmetic remain checked f64 operations, not interval certification.

`NurbsPolygonMesh` now supplies `vertex_nodes` and `normal_sides` alongside
original UVs and per-triangle bounds. Display duplicates at a crease share a
canonical geometric node and exactly the same cached position, while source
partials use explicit Left/Right limits. Two crossing creases can produce four
normal variants of one node. A trim boundary on a crease uses only the retained
side. Topology comparisons must use `vertex_nodes` when comparing display edges;
normal-duplicate indices intentionally differ. Nodes are keyed by construction
identity, not spatial welding or tolerance snapping.

Invalid precision, exhausted triangle/depth budgets, unresolvable intersections
or UV midpoints, reversed/collapsed UV triangles, singular sampled normals and
collapsed geometric triangles fail explicitly. Clipping work is limited to
16 million visited triangles; derivative work, source extraction and boundary
identity limits remain. Display permits at most 65536 triangles and ten uniform
subdivision levels. Full-source bounds are conservative and do not skip removed
regions. Global regularity/injectivity and cross-face sewing are not certified.

```sh
cargo run --example nurbs_polygon_crease > rational-creases.obj
```

The example exports actual face-derived positions, explicit crease normals and
OBJ face/normal indices. Independent tests check piecewise rational formulas,
per-triangle bounds, analytic one-sided normals, shared geometric nodes and
opposing interior edge traversal, UV area, no triangle crossing, four normals at
a crease intersection, orientation reversal, boundary-on-crease ownership and
invalid topology/precision/budgets. Existing single-patch and C1 tests remain.
Native/WASM builds share the kernel; no dedicated browser export is added here.

Holes, concave/general trim loops, sewing, closed rational solids and NURBS STEP
remain future work. No dependency or OCCT source was added. Original code:
MIT OR Apache-2.0.
