# Full-ellipse planar B-rep trims

![Actual WASM planar ellipse-cap clipping demo](ellipse-planar.png)

Planar faces can retain exact ellipse boundaries through
`PCurve::EllipseArc { center, cosine, sine, sweep }`. The UV curve is
`center + cosine*cos(t) + sine*sin(t)` and shares the owning
`Curve::EllipseArc` angle, including reversed coedge traversal. The axes need
not be orthogonal. Original code is MIT OR Apache-2.0; no OCCT source or new
dependency was used.

## Supported domain

The first supported face contains one complete ellipse wire represented by two
half arcs, each with sweep `pi`, opposite axes and a common center. Both coedges
must traverse consistently. Axis conditioning, finite coefficients, closure,
area/orientation and the owning 3D edge/pcurve relationship are checked. A
conservative coefficient-error sum bounds that relationship over the entire
arc, in addition to the existing sample checks. The 3D/pcurve coefficients must
also agree within checked binary64 arithmetic precision. Centers and axes of the two
halves must agree within an analogous whole-curve tolerance bound. The two
conics must also agree at checked binary64 arithmetic precision; nearly
coincident but distinct ellipses are rejected even when their difference fits
the model tolerance. Their accepted arithmetic deviation is included in
distance and root precision guards.

[Minor ellipse arcs with a straight closing chord](ellipse-segment-planar.md),
including the [half ellipse and diameter](half-ellipse-planar.md), are also supported.
[Concentric homothetic full-ellipse holes](ellipse-annulus-planar.md) are supported.
[Offset homothetic ellipse holes](ellipse-eccentric-planar.md) are supported too.
Other mixed loops, multiple/nonhomothetic holes, more general arc partitions and
ellipse-face subdivision are not yet supported. Invalid topology and unsupported
trim domains fail before query/bounds shortcuts. Independent ellipse pcurve
values do not imply support for arbitrary ellipse-trimmed modeling operations.

## Analytic queries and metrics

`clip_line_to_planar_face` maps an in-plane line through the inverse ellipse-axis
matrix and solves unit-circle intersections. It returns the proper interior
interval with original line parameters and owning ellipse edge parameters.
`intersect_planar_faces` can therefore intersect this ellipse face with existing
supported polygon/circular faces using the same shared-parameter segment API.
Tangencies, near tangencies, vertex/near-vertex hits, off-plane lines, lost
coordinate precision and ill-conditioned axes return errors. A conservative
lower singular-value bound converts the physical tolerance into the circle
root guard. No boundary snapping or display-mesh clipping occurs.

`classify_point_in_solid` accepts the ellipse face as part of a validated closed
B-rep. Inverse-axis coordinates give projected material membership; physical
boundary-band decisions refine ellipse chords using the interpolation bound
`(|cosine|+|sine|)*angular_width^2/8`. Actual ellipse samples provide upper distance
bounds. Refinement and checked floating-point margins follow the
[harmonic boundary classifier](harmonic-classification.md); unresolved distances
return errors. The ray classifier still requires two independent resolved rays.

Planar area uses Green's theorem. For translated center `C`, axes `A,B` and sweep
`s`, the signed arc contribution is

`[cross(C,A)*(cos(s)-1) + cross(C,B)*sin(s) + cross(A,B)*s] / 2`.

Face flux therefore includes the exact elliptical area in analytic solid volume.
Tessellation samples the original 3D ellipse edge with its bounded chord count;
the neighboring circular wall shares that count, producing conforming boundaries.
Planar triangulation uses sampled display contours only after B-rep validation.

## Actual closed-solid fixture and demo

```sh
cargo run --locked --example ellipse_planar -- 14 0
cargo run --locked --example ellipse_planar -- -14 0.37
./scripts/build-web.sh
```

`ellipse_planar_demo_solid(radius, height, slope, tolerance)` constructs a
restricted closed B-rep fixture: a two-arc cylinder based at `z = -height/2`,
retaining the material below `z = -slope*x` and closing it with the exact ellipse
cap. Dimensions must be resolved and positive. The plane must stay strictly
between both full rims, with the oblique-subdivision tolerance margin; touching
or crossing a rim fails. The retained volume is `pi*radius^2*height/2`. Shared
edge/vertex indices are compacted and the resulting solid is validated.
This fixture constructor is not a general-purpose solid partition API.

The default fixture has radius 24 mm, height 24 mm and slope 0.25. Its four faces
and six edges enclose `21714.688421613... mm³`. The cap ellipse measures
approximately 49.5 × 48 mm. Example arguments are the in-plane V offset and rigid
rotation in radians. `hagane_ellipse_planar_demo` exposes the same fixture and
queries through the existing WASM JSON/error buffer contract.

Serve `web/` using the README instructions, open `intersections.html`, and select
**Planar ellipse cap**. The displayed open face mesh is extracted from the actual
closed B-rep's tessellation; the volume refers to that closed parent. Cyan marks
the in-plane line and resolved boundary events. Tangent and vertex probes show
explicit unresolved errors; an exterior line returns an empty clip. Orbit and
zoom work as before.

Tests check independent roots, reversal, edge parameters, ellipse/polygon face
intersections, cap/wall/material classification, both tilt signs, rotation, tiny
dimensions, nonorthogonal axes, conditioning, tangent/vertex rejection, malformed
pcurves and unsupported domains. Analytic volume and closed oriented tessellation
edges are checked. Native/WASM meshes and queries agree; browser tests cover the
selected cap, errors, recovery and orbit. References are inverse affine-circle
coordinates, Cartesian line/circle roots, Green's theorem, Taylor interpolation
error and the Hausdorff-distance inequality, recorded in
[provenance](references.md). General capped partitions and curved Booleans remain
future work.
