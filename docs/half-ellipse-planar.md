# Diameter-closed half-ellipse planar trims

![Actual WASM half-ellipse clipping demo](half-ellipse-planar.png)

The planar trim validator accepts one wire with exactly two coedges: an
`EllipseArc` pcurve of sweep `pi`, followed by an affine line pcurve closing
its diameter. Each keeps its owning 3D edge parameter and traversal direction.
Endpoints must agree at checked binary64 precision as well as model tolerance;
finite axes, conditioning, closure, surface correspondence and orientation
remain checked. [Minor arcs with non-diameter chords](ellipse-segment-planar.md) are also supported.
Other mixed loops, ellipse holes and general curved face subdivision return errors.

Inverse ellipse coordinates identify the region as the unit disk intersected
with `y >= 0`. In-plane clipping combines circle roots on the retained half
with a straight-diameter intersection. Returned intervals use the original
line parameter; ellipse events retain angles and diameter events retain the
original line-edge parameter. Tangencies, vertex hits, near contacts, diameter
overlap and unresolved precision return explicit errors. Physical boundary
checks combine the diameter segment distance with bounded ellipse chord
refinement; classification uses the same validated domain and resolved rays.
Area uses the existing exact Green's theorem integrals. Display triangulation
samples the B-rep and shares edge samples with adjoining faces.

```sh
cargo run --locked --example half_ellipse_planar -- 0 14 0
cargo run --locked --example half_ellipse_planar -- 1 0 0.37
./scripts/build-web.sh
```

Arguments are query mode, in-plane offset and rigid rotation in radians.
Modes 0, 1 and 2 respectively cross the curved boundary, run from the diameter
to the arc, and reverse that direction. WASM exposes the same query through
`hagane_half_ellipse_planar_demo` and the existing JSON/error buffer.

`half_ellipse_planar_demo_solid(radius, height, slope, tolerance)` extrudes an
exact semicircle-plus-diameter profile from `z = -height/2`, retains the material
below `z = -slope*x`, and closes it with the exact half-ellipse cap. The plane
must stay strictly between the source rims. This restricted fixture is not a
general solid partition operation. The retained analytic volume is
`pi*radius^2*height/4`. The default radius 24 mm, height 24 mm and slope 0.25
produce four faces and six edges enclosing `10857.344210806... mm³`.

Open `intersections.html` and choose **Half ellipse and diameter**. The visible
face mesh comes from the closed solid's tessellation. The controls show arc/line
crossings, empty results and explicit contact errors; orbit and zoom work.
Native tests cover volume, material/boundary classification, opposite slopes,
rotation, tiny dimensions, original edge parameters and closed oriented meshes.
WASM tests compare native output with independent analytic roots, and browser
tests exercise controls, provenance, rejected contacts and recovery.

The implementation adds no dependencies and copies no OCCT code. Original code
is MIT OR Apache-2.0. Mathematical sources are inverse affine-circle coordinates,
line/segment intersections and Green's theorem, with the existing bounded
interpolation and distance arguments recorded in [references](references.md).
