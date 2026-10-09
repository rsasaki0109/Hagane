# Convex UV polygon faces

`NurbsPolygonFace` retains an open trimmed NURBS face with a single strictly
convex, counterclockwise straight-UV outer wire (3–64 corners). Exact rational
lifted edges, shared vertices and affine same-parameter pcurves come from
`NurbsSurfaceWire`. Face orientation is independently +1 or -1. The original
supporting surface remains intact; the wire defines the retained region.
Validation checks both copies of the supporting geometry, face wire references,
traversal and canonical boundary geometry. No surface fitting or mesh Boolean
is used. Boundary-only validation does not certify global regularity/injectivity.

Interior display is deliberately scoped: `tessellate_affine(error, tolerance)`
accepts only one degree-(1,1), four-control-point, equal-weight patch with a
parallelogram control net within the engineering arithmetic reserve. It evaluates
original UV corners and emits a convex fan with orientation-correct normals.
The requested error must cover twice the residual bilinear twist plus the
world-coordinate arithmetic reserve. This guard is engineering floating-point
validation, not interval certification. Curved patches, additional knot spans,
nonuniform weights, singular normals, unresolvable precision and degenerate
triangles fail explicitly. Curved faces can still be constructed and validated;
this does not imply their interior display is implemented.

```sh
cargo run --example nurbs_polygon_face > triangle.obj
```

The example writes the actual face-derived display geometry as Wavefront OBJ.
Native tests check the independent analytic triangle area, both orientations,
retained curved B-rep, unsupported display and corrupted entities. Native and
WASM share the implementation; no browser UI or dedicated WASM export was added.

Holes, concave/general trim loops, bounded curved-interior tessellation, sewing,
closed rational solids and NURBS STEP remain unsupported. Mathematical convex
fan triangulation is implemented independently, with no new dependency or OCCT
source. Original code: MIT OR Apache-2.0.

A subsequent [bounded bilinear display](nurbs-polygon-bounded.md) API supports
genuinely curved positive-weight rational saddle patches; the affine API retains its original
contract. Higher-degree and multi-span rational polygon display remain unsupported.
