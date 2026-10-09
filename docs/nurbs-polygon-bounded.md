# Bounded bilinear polygon display

`NurbsPolygonFace::tessellate_bilinear_bounded` displays the actual retained
convex UV polygon on one degree-(1,1), four-control-point, positive-weight rational patch.
Genuinely curved saddle patches are supported. The exact rational boundary and
supporting B-rep remain unchanged; the display never substitutes a mesh Boolean.

The returned `NurbsPolygonMesh` contains original vertex UVs, indexed geometry
and one engineering error bound per triangle. A convex fan is uniformly
subdivided by edge midpoints until every triangle satisfies the requested error.
Midpoints are keyed by shared edge indices, so neighboring triangles share nodes
and have no hanging vertices. Boundary nodes subdivide only original polygon edges; parameter roundoff is
included in the arithmetic reserve. Unresolved or reversed UV triangles are
rejected. Face orientation reverses triangles and normals together.

In normalized patch coordinates, the rational surface is `S=H/W`, with
bilinear homogeneous numerator H and positive bilinear denominator W.
Weights are normalized by their maximum, and spatial controls are recentered
before computing derivative bounds. Positive Bernstein weights give
`W >= min(weights)` and keep S inside the Euclidean control hull. Bounds on H
and W derivatives then follow from their control differences. Differentiating
`H=W*S` yields, for example, `Suu=-2*Wu*Su/W` and
`Suv=(Huv-S*Wuv-Su*Wv-Sv*Wu)/W`. The same equations hold for v derivatives.

Barycentric interpolation cancels first-order Taylor terms. With normalized
triangle rectangle widths du,dv, the spatial remainder is bounded by
`0.5*(bound(Suu)*du^2 + 2*bound(Suv)*du*dv + bound(Svv)*dv^2)`.
An engineering reserve covers world-coordinate, weight conditioning and
parameter arithmetic; nonfinite or unresolvable bounds are rejected. Equal
weights reduce this expression to the previous bilinear twist bound. This is
not formal interval certification or a global surface regularity certificate.

Only a single positive-weight bilinear patch is supported. Higher degrees and
multiple source spans explicitly return Unsupported. The budget is 1–65536 triangles and ten subdivision levels. Invalid precision,
exhausted budgets, unresolved UV midpoints, singular sampled normals and
numerically degenerate triangles return errors rather than partial success.
Source boundary construction and its existing identity guards still apply.

```sh
cargo run --example nurbs_polygon_bounded > saddle.obj
```

The example exports the actual curved face display as OBJ and reports triangle
count/maximum bound on stderr. Native tests independently evaluate the bilinear
formula at dense barycentric points (including independently weighted rational
evaluation and common weight factors 1e100/1e-100), check every reported bound, UV area and
coverage, boundary edges, opposing shared-edge traversal, reversed normals and
explicit failures. The implementation builds for native and WASM; a dedicated
browser page/WASM callable export is not yet provided.

Higher-degree/multi-span rational polygon display, holes, concave boundaries, cross-face
sewing, closed rational solids and NURBS STEP remain future work. No dependency
or OCCT source was added. Original code: MIT OR Apache-2.0.
