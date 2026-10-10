# Finite segments through bored oblique children

`NurbsBoredObliqueFrustumSolid::intersect_segment(start, end, policy)` queries
actual retained material, returning sorted boundary hits and zero, one or two
material intervals in the original segment parameter `[0, 1]`.

The checked original bored-solid intersection runs first. Each original material
interval is clipped independently against the actual retained cut halfspace.
A plane root in the hole is not a cut-cap hit. Both interval endpoints must
prove a fully retained or removed interval; a midpoint alone cannot certify an
empty result. Interior segment endpoints introduce no synthetic boundary hits.

Hits retain actual face identity, surface UV and outward normal. Restricted
outer and cavity walls preserve angular U and convert physical generator height
to the actual rational V parameter. Cut-cap UV comes from its actual affine
plane. Surface residual, boundary classification and entry/exit orientation
are checked before returning success. No mesh intersection is used.

The workflow read-only `segment` command uses the accepted selected child:

```json
{"command":"segment","component":0,"start":[-20,0,6],"end":[20,0,6]}
```

For the default bored-oblique example's lower child, this returns material
intervals `[0.15, 0.4]` and `[0.6, 0.85]`, preserving the cylindrical void.
Browser rejection retains the last accepted query, geometry, camera and STEP.

Finite distinct endpoints and resolved transverse intersections are required.
Tangency, boundary endpoints, close shared rims, unresolved near-parallel plane
crossings, tiny material intervals and insufficient coordinate precision return
explicit errors. Precision bands use the original complete source extent.
Inherited source intersection restrictions apply even on a removed side.
Infinite line/ray queries and repeated cuts remain unsupported for this type.

Tests cover independent analytic interval oracles, both children, cut roots in
material and void, rotations, multiple scales, parameter reversal, actual UV
residuals and normals, contact, tiny slivers and invalid-input recovery.
Original code is MIT OR Apache-2.0; no dependency or OCCT source is added.
