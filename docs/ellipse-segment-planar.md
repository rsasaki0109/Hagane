# Minor ellipse arcs closed by straight chords

![Actual WASM 90-degree ellipse/chord clipping demo](ellipse-segment-planar.png)

A planar B-rep face can contain one exact ellipse arc with a sweep in `(0, pi]`,
followed by one straight closing chord. This extends the existing
[half ellipse and diameter](half-ellipse-planar.md) domain. The pcurve axes can
be nonorthogonal; each coedge retains its owning 3D edge parameter and direction.
The ellipse coedge must be first in the two-coedge wire. More than one arc,
holes, sweeps above `pi` and general curved face subdivision remain unsupported.

## Checked domain and algorithms

Finite coefficients, well-conditioned axes, endpoint agreement, closure,
surface correspondence and orientation are checked before queries. Endpoints
must agree at checked binary64 precision as well as model tolerance. Let
`sigma_lower` be the conservative lower singular-value bound for the ellipse
axis matrix. The segment is rejected when its thickness lower bound
`2*sin(sweep/4)^2*sigma_lower` is at most ten model linear tolerances. Thus a tiny
angle does not silently produce collapsed or unresolved geometry.

Inverse ellipse coordinates give the unit disk intersected with the half-plane
`dot([cos(sweep/2), sin(sweep/2)], q) >= cos(sweep/2)`.
The exact semicircle case retains the plane `q.y >= 0`. Line clipping combines
circle roots on the retained arc with a finite-chord intersection. It returns
one convex material interval, with original line parameters, ellipse angles and
straight-edge parameters. Tangencies or near tangencies to the supporting
ellipse, chord overlap or near overlap, vertex contacts, insufficient precision
and unresolved crossing counts return explicit errors. No mesh clips geometry.

Physical boundary tests combine point-to-chord distance with bounded ellipse
interpolation over the actual arc interval. Classification uses this validated
region and checked independent rays. Existing Green's theorem integrals give
analytic area and volume. The tessellator samples the original ellipse and shares
those samples with adjoining walls; tests independently sample the section
against display chords and check closed, oriented mesh edges.

## Closed-solid fixture and demo

```sh
cargo run --locked --example ellipse_segment_planar -- 1.5707963267948966 0 20 0
cargo run --locked --example ellipse_segment_planar -- 2.5 1 0 0.37
./scripts/build-web.sh
```

Arguments are arc sweep in radians, query mode, in-plane offset and rigid
rotation in radians. Modes 0, 1 and 2 cross the arc, run from chord to arc, and
reverse that line respectively. `hagane_ellipse_segment_planar_demo` uses the
same arguments and existing WASM JSON/error buffer.

`ellipse_segment_planar_demo_solid(radius, height, slope, sweep, tolerance)`
extrudes a circular-segment profile symmetric about positive Y from
`z = -height/2`, retains the material below `z = -slope*x`, and closes it with
the exact ellipse/chord cap. The plane must stay strictly between the circular
source rims. This restricted fixture is not a general solid partition API.
Symmetry makes the retained analytic volume
`radius^2*height*(sweep - sin(sweep))/4`, independent of the accepted tilt.
The default radius 24 mm, height 24 mm, slope 0.25 and sweep `pi/2` produce four
faces and six edges enclosing approximately `1972.6721054 mm³`.

Serve `web/` following the README and open `intersections.html`. Select
**90° ellipse arc and chord** to inspect the actual cap tessellated from its
closed B-rep parent. Chord/arc probes show original edge parameters; tangent
probes report errors and an exterior line returns an empty result. Orbit and
zoom remain available.

Tests cover independently computed roots, bounds and volume across several
minor sweeps, reversal, rigid placement, positive/negative slopes, microscopic
sizes, classification, nonorthogonal axes, physical chord bands, chord error,
closed meshes, unresolved thickness and contact rejection. Native/WASM queries
and meshes agree, and browser controls exercise errors and recovery.

Original code is MIT OR Apache-2.0, adds no dependencies and uses no OCCT source.
The mathematical basis is affine circle coordinates, circle segments,
line/segment intersections, Green's theorem and the existing interpolation bound,
recorded in [references](references.md).
