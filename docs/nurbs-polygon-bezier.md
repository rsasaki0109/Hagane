# Bounded high-degree rational polygon display

`NurbsPolygonFace::tessellate_bounded` displays a retained convex straight-UV
polygon on one positive-weight tensor rational Bezier patch. Higher degrees,
including biquadratic patches and asymmetric cubic/linear patches, are supported.
The previous `tessellate_bilinear_bounded` entry point retains its degree-(1,1)
contract and delegates to the shared implementation. Affine display retains its
original contract as well.

For the single-span case the surface contains exactly `(p+1)*(q+1)` control
points. The current API additionally accepts structurally C1 spans through the
[whole-source derivative extension](nurbs-polygon-multispan.md). C0 knots are
rejected. Source degree/control limits and exact boundary construction
limits still apply. In particular a diagonal lifted boundary has degree p+q and
must fit the curve degree limit of 16; a constant UV axis contracts that degree.
Nonrepresentable or poorly conditioned inputs may be rejected even when they
are mathematically valid.

Homogeneous controls are recentered and weights normalized. Bernstein derivative
control nets use scaled first and second differences, and their maximum norms
bound derivatives on the normalized parameter rectangle. Positive weights give
a lower bound on W. Differentiating H=W*S yields
`Suu=(Huu-S*Wuu-2*Su*Wu)/W`, the analogous v identity, and
`Suv=(Huv-S*Wuv-Su*Wv-Sv*Wu)/W`. The Euclidean control hull bounds S relative to
the chosen origin. These bounds supply the existing barycentric Taylor remainder
for each triangle. Degree, weight conditioning and UV/world arithmetic reserves
are checked; this is engineering floating-point validation, not interval proof.
Nonfinite derivative controls/bounds fail explicitly.

Conforming uniform midpoint subdivision preserves shared geometric indices,
original UVs and boundary traversal. Output contains one error bound per triangle,
with a 65536-triangle limit and ten levels. Degenerate/reversed UV triangles,
singular sampled normals and collapsed display triangles fail. Global surface
regularity/injectivity is not certified. Higher curvature or conditioning can
exhaust the conservative global-bound budget without producing a partial mesh.

```sh
cargo run --example nurbs_polygon_bezier > rational-biquadratic.obj
```

This exports the actual B-rep-derived display of a weighted biquadratic patch
with a diagonal convex boundary. Native tests independently compute Bernstein
rational formulas at dense barycentric samples for degrees (2,2), (3,1), (1,3),
in nonunit UV domains, checking reported bounds; existing UV coverage, shared-edge, orientation,
weight-scale, precision and failure tests remain. The kernel builds for native
and WASM; no dedicated browser export was added in this milestone.

C0 polygon interiors, holes, concave/general trim curves, sewing,
closed rational solids and NURBS STEP remain future work. No dependency or OCCT
source was added. Original code: MIT OR Apache-2.0.

The subsequent [C1 multi-span display](nurbs-polygon-multispan.md) extension
now accepts structurally C1 source knots through the general bounded API. C0
polygon interiors remain unsupported.
