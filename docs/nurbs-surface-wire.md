# Convex UV surface wires

`NurbsSurfaceWire` closes 3–64 straight UV segments on a retained NURBS
surface. Each segment is the exact rational composition described in
[surface edges](nurbs-surface-edge.md), rather than a fitted 3D curve.
Corners are shared vertex indices; coedges retain affine pcurves over T=0..1.
Input traversal is preserved, including clockwise loops. Do not repeat the
first corner at the end.

The initial domain is strictly convex simple UV polygons. Exact binary64
orientation signs require every nonincident corner to be strictly on the same
side of every directed edge. This rejects self intersections (including star
polygons), concavity, repeated corners and collinearity. No physical-length
tolerance is substituted for UV orientation. Each lifted edge independently
applies the existing parameter resolution, work, degree and physical identity
guards. Thus mathematically valid poorly conditioned inputs may be rejected.
No global surface injectivity or regularity is certified.

`validate` checks canonical rational geometry, vertices, shared closing indices,
pcurves and traversal, including publicly modified entities.
`tessellate_boundary` returns separate bounded polylines with original edge
parameters; its segment limit is per edge and existing engineering arithmetic
guards apply. This is boundary display only. It does not triangulate the interior,
construct a trimmed face, sew faces or produce a closed solid. Polygonal face
interior tessellation and inner-wire containment remain subsequent work.

Run the native demonstration:

```sh
cargo run --example nurbs_surface_wire
```

No dependencies were added. Convex supporting-half-plane characterization and
the kernel's independent exact orientation predicate are used; no OCCT source
was copied or translated. Original implementation: MIT OR Apache-2.0.
