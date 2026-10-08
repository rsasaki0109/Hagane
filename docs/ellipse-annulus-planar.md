# Planar ellipse caps with concentric holes

This document describes the original homothetic subdomain and its fixtures.
The current validator also accepts unequal-axis/rotated complete ellipse holes
under a conservative enclosing-circle certificate; see
[the expanded supported domain](tilted-bore.md). Homothety requirements below
apply to the original fixtures, not to every currently supported planar hole.


![Actual WASM ellipse cap and hole clipping demo](ellipse-annulus-planar.png)

A planar B-rep face can contain one complete outer ellipse and one strictly
interior, concentric, homothetic full-ellipse hole. Each wire has two pi-sweep
ellipse coedges with opposite axes. Both wires retain the actual owning
3D ellipse edge parameters and traversal directions. The outer wire is CCW
and the hole is CW in the supporting plane; inward wall orientations are
included when constructing the shared cap coedges.

## Supported domain and checks

The hole's center must match the outer center at checked binary64 precision.
The first arc axes must be positive scalar multiples of the outer axes, or
both must be negated scalar multiples. Nonorthogonal axes are accepted when
well conditioned; arbitrary relative phase, unequal axis ratios and mixed ellipse/chord regions
with holes are not supported. [Multiple disjoint homothetic holes](ellipse-multi-hole-planar.md)
are supported as an extension.
[Offset homothetic holes](ellipse-eccentric-planar.md) are also supported, with
a checked containment bound. The existing single-wire ellipse and minor arc/chord domains remain.
Invalid or unsupported trim geometry fails before empty-query shortcuts.

Finite coefficients, whole-arc 3D/pcurve agreement, matching half conics,
closure and orientation are checked as before. Let `k` be the hole/outer scale
ratio and `sigma_lower` the conservative lower singular-value bound of the
outer axis matrix. A clearance lower bound `(1-k)*sigma_lower` must exceed
ten model linear tolerances plus the accepted homothety coefficient deviation.
Nearly touching or indistinguishable boundaries return errors. Centers and
axes that differ beyond checked arithmetic precision are rejected even when
they fit the model tolerance.

In-plane clipping solves each wire analytically, preserves original line and
ellipse parameters, sorts the shared-parameter events, and verifies outer/inner/
inner/outer order. A line crossing the hole returns two material intervals and
four boundary events; a line missing the hole returns one interval. Tangencies,
near tangencies, vertex contacts, unresolved event separation and lost coordinate
precision return errors. Planar-face intersections use these same intervals and
can return two exact segments against a polygon face.

Solid classification subtracts the inner ellipse's material membership from the
outer ellipse and checks physical boundary bands on both curves, using the
existing bounded chord-distance refinement. It does not classify display meshes.
Green's theorem retains the exact signed hole area in analytic volume.
Tessellation shares original boundary samples with both outer and inward walls.
Display triangulation uses the hole contour after B-rep validation.

## Closed tube fixture and browser demo

```sh
cargo run --locked --example ellipse_annulus_planar -- 6 0
cargo run --locked --example ellipse_annulus_planar -- -6 0.37
./scripts/build-web.sh
```

Arguments are in-plane V offset and rigid rotation in radians.
`hagane_ellipse_annulus_planar_demo` exposes the same native operation through
the existing WASM JSON/error buffer.

`ellipse_annulus_planar_demo_solid(radius, inner_radius, height, slope, tolerance)`
extrudes a concentric two-arc tube from `z=-height/2`, retains material below
`z=-slope*x`, and closes the top with its exact ellipse annulus. The cutting
plane must remain strictly between the source circular rims. This is a restricted
fixture constructor, not a general curved solid partition operation. Its
analytic retained volume is `pi*(radius^2-inner_radius^2)*height/2`.
The default outer radius 24 mm, inner radius 12 mm, height 24 mm and slope 0.25
produce six faces, twelve edges and approximately `16286.0163162095 mm³`.

Serve `web/` as described in the README, open `intersections.html`, and select
**Ellipse cap with a hole**. The visible face is extracted from the closed
B-rep's tessellation. Offset 6 mm crosses both boundaries; 18 mm misses the hole;
12 mm produces an explicit tangent error, and 30 mm returns an empty result.
Orbit and zoom remain available. The displayed volume belongs to the parent solid.

Native tests verify independent roots, volume and bounds, hole/material/boundary
classification, both tilt signs, rotation, microscopic dimensions, original edge
parameters, polygon-face intersections, nonorthogonal axes, strict clearance and
rejection of unsupported holes. Mesh checks cover oriented closure, sampled
ellipse chord error on both walls and absence of triangles filling the hole.
Native/WASM meshes and queries agree; browser checks cover both interval counts,
hole provenance, errors, recovery and orbit.

This independently authored implementation is MIT OR Apache-2.0 and adds no
libraries or OCCT source. Mathematical sources are affine circle coordinates,
singular-value clearance bounds, shared-parameter interval subtraction, Green's
theorem and the existing bounded interpolation/distance arguments, recorded in
[references](references.md).
