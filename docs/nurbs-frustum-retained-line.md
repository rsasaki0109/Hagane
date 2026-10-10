# Lines and rays through retained frustum material

Actual `NurbsBoredFrustumSolid` and `NurbsObliqueFrustumSolid` support
`intersect_line(origin, direction, policy)` and `intersect_ray(...)`.
Both return `NurbsFrustumMaterialLineIntersection`, containing actual kept-face
hits and `material_intervals`. Bored material may have two separated intervals;
oblique material is convex and has at most one.

Parameters always refer to the original world input `origin + t*direction`.
Direction need not be unit length. Lines permit negative parameters; rays keep
`t >= 0`. An inside ray starts its material interval at 0 without inventing a
boundary hit there. A ray originating in the hole can enter the inner wall and
leave the outer wall.

For source `[16,8]`, height 24 and coaxial hole radius 4, origin `[-20,0,12]`
and direction `[2,0,0]` give line hits `4,8,12,16` and material intervals
`[4,8]`, `[12,16]`. Origin `[0,0,12]` with that direction gives the ray interval
`[2,6]`; origin `[6,0,12]` gives `[0,3]`.

The implementation validates the actual body and guards the original anchor,
world coordinates and frame metric before cropping or an empty-result shortcut.
It crops a unit-speed line to an expanded finite enclosure of the source body,
then queries the actual retained segment. The source enclosure contains both
retained body kinds; it never replaces their surfaces or material. Original
parameters are recovered with checked reconstruction. Each returned face/UV
witness is evaluated on the actual retained B-rep, and positive interval width,
order and actual interior membership are checked.

Ray clipping uses actual origin classification and processes every material
interval. Contact proximity is measured as physical travel, not raw parameter
magnitude; scaling direction therefore does not change geometric admission.
Boundary ray origins, tangencies, unresolved cap/rim contacts, near-parallel
cut contacts, tiny intervals, parameter overflow and insufficient original
anchor precision return explicit errors. Existing finite-query restrictions
remain. Canonical frustum line/ray APIs and their optional interval type remain
unchanged.

The [workflow](nurbs-frustum-workflow.md) exposes read-only `line` and `ray`
commands using the accepted component and document tolerance. Browser probes
retain camera, geometry, STEP, history and other successful query results.
Actual rigid poses use the same kernel in native and WASM builds.

This reuses Hagane's independent checked slab enclosure and finite intersections;
no OCCT source, external implementation or new dependency is introduced.
Original code is MIT OR Apache-2.0.
