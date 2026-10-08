# Planar ellipse caps with offset holes

![Actual WASM eccentric ellipse-hole clipping demo](ellipse-eccentric-planar.png)

A complete ellipse outer wire now accepts one strictly interior, aligned
homothetic complete ellipse hole with a different center. This extends the
[concentric annulus](ellipse-annulus-planar.md) domain. Both wires retain their
two pi-sweep owning ellipse edges, pcurves, traversal and surface orientation.
The hole axes must still be equal positive multiples of the outer axes, or
both negated multiples. Nonorthogonal well-conditioned axes remain supported.
Multiple holes, unequal axis ratios, arbitrary relative arc phase and holes in
arc/chord regions remain unsupported.

## Containment and exact geometry

Mapping the hole center through the inverse outer axis matrix gives offset `q`.
If its axis scale ratio is `k`, strict containment requires `norm(q)+k < 1`.
The physical clearance lower bound is
`(1-k-norm(q))*sigma_lower`, where `sigma_lower` bounds the outer axis matrix's
smallest singular value. This bound must exceed ten model linear tolerances,
the axis coefficient deviation and a checked arithmetic margin. Touching,
nearly touching, exterior or unresolved holes return errors. Axis homothety is
checked at binary64 precision; center offsets are actual retained geometry.

Planar ellipse edge/pcurve coefficients now also have to agree at checked
arithmetic precision, in addition to model tolerance, over the entire curve.
A small pcurve-only displacement cannot masquerade as a correctly offset 3D hole.
No snapping or mesh geometry changes are used to establish containment.

The existing analytic per-wire roots and sorted shared-parameter event order
produce one or two material intervals with original line and ellipse parameters.
Classification subtracts the displaced inner region and checks physical boundary
bands. Signed Green's theorem area/volume and B-rep-derived conforming meshes use
the actual hole position. Tangencies, vertex contacts and insufficient precision
retain their explicit error contracts.

## Actual closed-solid fixture and demo

```sh
cargo run --locked --example ellipse_eccentric_planar -- 6 4 0
cargo run --locked --example ellipse_eccentric_planar -- -6 4 0.37
./scripts/build-web.sh
```

Arguments are hole center X in millimeters, line V offset and rigid rotation in
radians. `hagane_ellipse_eccentric_planar_demo` exposes the same query through
the existing WASM JSON/error buffer.

`ellipse_eccentric_planar_demo_solid(radius, inner_radius, height, slope, center,
tolerance)` accepts a source XY hole center `[cx,cy]`, extrudes the circular
profile from `z=-height/2`, retains material below `z=-slope*x`, and closes it
with an exact ellipse cap. The plane must stay strictly between all source rims.
This restricted fixture is not a general curved partition operation.
Its retained analytic volume is

`pi*(radius^2-inner_radius^2)*height/2 + slope*pi*inner_radius^2*cx`.

The second term comes from the removed hole's X first moment; moving the hole
changes the volume of an obliquely capped part. Default outer radius 24 mm,
inner radius 8 mm, height 24 mm, slope 0.25 and center `[6,0]` produce six faces,
twelve edges and approximately `19603.5381584 mm³`.

Serve `web/` following the README and open `intersections.html`. Select
**Ellipse cap with an offset hole**. Offset 4 mm crosses both ellipses, 14 mm
misses the hole, 8 mm reports a tangent error and 30 mm returns an empty result.
The displayed face is extracted from its closed B-rep parent's tessellation;
the volume is that of the parent solid. Orbit and zoom work as before.

Native tests check independent displaced roots and first-moment volume, XY
center offsets, tilt signs, rotation, tiny dimensions, hole/material/boundary
classification, oriented closed meshes and no triangles filling the hole.
Nonorthogonal trim membership, strict containment, contact rejection and
pcurve-only coefficient corruption are checked. WASM tests verify independent
roots and volumes for several center positions and native query/mesh parity;
browser tests cover asymmetric intersections, interval counts, errors, recovery
and orbit.

The independently authored code is MIT OR Apache-2.0, adds no dependencies and
uses no OCCT source. Mathematical sources are affine circle coordinates,
triangle-inequality/singular-value clearance bounds, circle area and first moments,
Green's theorem and the existing root/distance arguments in [references](references.md).
