# C1 multi-span rational polygon display

`NurbsPolygonFace::tessellate_bounded` now displays convex straight-UV polygons
crossing structurally C1 or smoother knots. Source NURBS geometry and exact
lifted boundary curves/pcurves remain intact. No fitting or mesh Boolean is used.

Each source span is extracted as a rational Bezier patch. Its homogeneous
Bernstein derivative control bounds are rescaled from local patch coordinates
to the original surface's normalized UV domain. Componentwise maxima provide
whole-source first/second derivative bounds. Along a triangle segment, a C1
piecewise-smooth surface has an absolutely continuous first derivative; integrating
the piecewise Hessian bound therefore gives the same barycentric Taylor remainder
across knot crossings. Second derivatives need not agree at the knots. Triangles
may cross C1 lines; normals are continuous there and need no crease splitting.

Every internal source knot must have multiplicity strictly below its axis degree.
Any structural C0 knot is rejected, even if the control geometry happens to be
smooth or the knot lies outside the retained polygon. C0 display requires explicit
boundary subdivision and one-sided normal topology, which this API does not yet
implement. Full-source derivative bounds/extraction are used; removed regions
are not skipped. Poor conditioning or curvature outside the retained polygon may
therefore exhaust the conservative display budget.

Existing boundary degree/identity limits, Bezier extraction limits, numerical
range checks and engineering arithmetic reserves apply. Derivative work is
preflighted at at most 16 million `patch_count * control_count_per_patch^2` units.
Display still permits 1–65536 triangles and ten uniform subdivision levels.
It rejects invalid or unresolvable precision, exhausted budgets, UV collapse,
singular sampled normals and collapsed display triangles. This is not formal
interval certification or a global regularity/injectivity proof.

```sh
cargo run --example nurbs_polygon_multispan > c1-polygon.obj
```

The example exports actual face-derived triangles spanning four rational source
patches in nonunit UV domains. Native tests independently evaluate Cox–de Boor
basis functions at dense barycentric points, check reported bounds and assert
that triangles really cross both U and V knots. Existing orientation, coverage,
shared-edge, corruption, precision, single-patch and weight-scale tests remain;
C0 source rejection is explicit. Native/WASM builds share the implementation;
no dedicated browser export was added in this milestone.

C0 polygon interiors, holes, concave/general trim loops, cross-face sewing,
closed rational solids and NURBS STEP remain future work. No dependency or OCCT
source was added. Original code: MIT OR Apache-2.0.
