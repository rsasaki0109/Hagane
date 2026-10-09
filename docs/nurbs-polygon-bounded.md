# Bounded bilinear polygon display

`NurbsPolygonFace::tessellate_bilinear_bounded` displays the actual retained
convex UV polygon on one degree-(1,1), four-control-point, equal-weight patch.
Genuinely curved saddle patches are supported. The exact rational boundary and
supporting B-rep remain unchanged; the display never substitutes a mesh Boolean.

The returned `NurbsPolygonMesh` contains original vertex UVs, indexed geometry
and one engineering error bound per triangle. A convex fan is uniformly
subdivided by edge midpoints until every triangle satisfies the requested error.
Midpoints are keyed by shared edge indices, so neighboring triangles share nodes
and have no hanging vertices. Boundary nodes subdivide only original polygon edges; parameter roundoff is
included in the arithmetic reserve. Unresolved or reversed UV triangles are
rejected. Face orientation reverses triangles and normals together.

In normalized patch coordinates, the polynomial is
`A + B*u + C*v + D*u*v`. The constant and affine terms cancel under barycentric
interpolation. After shifting to each triangle's UV bounding rectangle, both
`u*v` and its interpolant lie between zero and the rectangle area. Therefore
`|D| * du * dv` bounds the spatial interpolation error in real arithmetic.
An engineering reserve covers world-coordinate and parameter arithmetic;
nonfinite or unresolvable bounds are rejected. This is not formal interval
certification or a global surface regularity certificate.

Only a single equal-weight bilinear patch is supported. Nonuniform rational
weights, higher degrees and multiple source spans explicitly return Unsupported.
The budget is 1–65536 triangles and ten subdivision levels. Invalid precision,
exhausted budgets, unresolved UV midpoints, singular sampled normals and
numerically degenerate triangles return errors rather than partial success.
Source boundary construction and its existing identity guards still apply.

```sh
cargo run --example nurbs_polygon_bounded > saddle.obj
```

The example exports the actual curved face display as OBJ and reports triangle
count/maximum bound on stderr. Native tests independently evaluate the bilinear
formula at dense barycentric points, check every reported bound, UV area and
coverage, boundary edges, opposing shared-edge traversal, reversed normals and
explicit failures. The implementation builds for native and WASM; a dedicated
browser page/WASM callable export is not yet provided.

General rational curved polygon display, holes, concave boundaries, cross-face
sewing, closed rational solids and NURBS STEP remain future work. No dependency
or OCCT source was added. Original code: MIT OR Apache-2.0.
