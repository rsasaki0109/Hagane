# Typed analytic intersections

The first general intersection APIs operate on analytic surfaces. They return
geometry and parameters for subsequent trim clipping and face splitting.
They do not modify a B-rep, split faces, or implement a general Boolean.
All calculations are independent pure Rust implementations shared with WASM.

## Plane/plane

```rust
use hagane::{Surface, Point3, Vec3, GeometryTolerance,
    intersect_plane_plane, PlanePlaneIntersection};
let xy = Surface::Plane {
    origin: Point3::new(0.0, 0.0, 3.0),
    u: Vec3::new(1.0, 0.0, 0.0), v: Vec3::new(0.0, 1.0, 0.0),
};
let yz = Surface::Plane {
    origin: Point3::new(5.0, 0.0, 0.0),
    u: Vec3::new(0.0, 1.0, 0.0), v: Vec3::new(0.0, 0.0, 1.0),
};
if let PlanePlaneIntersection::Line(line) =
    intersect_plane_plane(&xy, &yz, GeometryTolerance::default())?
{
    let point = line.evaluate(2.0)?; // (5, 2, 3)
    let first_uv = line.first.evaluate(2.0); // (5, 2)
    let second_uv = line.second.evaluate(2.0); // (2, 3)
}
```

The output is `Line`, `Parallel` or `Coincident`. Classification uses the
configured angular budget and a length budget at the origin-separation scale.
Parallel/coincident are policy classifications, not exact predicates for
mathematical equality. Plane bases must be finite and orthonormal under the
same checks as line/plane intersection.

The intersection direction is the normalized cross product of the ordered
plane normals. Its anchor is the intersection point closest to the first
plane's origin; reversing inputs reverses direction and may change the anchor.
`PlaneIntersectionLine` has an infinite signed **length** parameter, and affine
pcurves on both planes evaluated at that same parameter. This is distinct from
the normalized [0,1] parameter of a finite `Curve::Line` B-rep edge. Trim clipping
must determine finite intervals and reparameterize edges/pcurves together.

The solve uses local origin separation, a direction within the first plane,
and a single scalar projection onto the second normal. Checked anchor and local
step evaluations verify both UV reconstructions. Overflow and loss of surface
agreement produce errors. No absolute world-coordinate plane constants are
squared or subtracted.

## Line/cylinder

```rust
use hagane::{Surface, Point3, Vec3, GeometryTolerance,
    intersect_line_cylinder, LineCylinderIntersection};
let cylinder = Surface::Cylinder {
    center: Point3::new(0.0, 0.0, 0.0), radius: 2.0, height: 4.0,
};
let hits = intersect_line_cylinder(
    Point3::new(-5.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 0.0),
    &cylinder, GeometryTolerance::default(),
)?;
if let LineCylinderIntersection::Points(points) = hits {
    // Two crossing hits: original line parameters 1.5 and 3.5.
    // Each hit includes a 3D point, cylinder UV, and crossing/tangent contact.
    assert_eq!(points.len(), 2);
}
```

This intersects an infinite line with the **bounded full lateral surface** of
a Z or framed cylinder. The line may be oblique and its direction need not be
unit length. Results are:

- `Empty`: no lateral hit in axial interval [0,height]. End disks are excluded.
- `Points`: one or two hits sorted by the original line parameter; each has
  `point`, `parameter`, `uv` and `Crossing`/`Tangent` contact. Axial clipping can
  leave one crossing hit. UV is angle in [0,2pi) and axial length in [0,height].
- `Coincident`: an exactly axial generator lies on the lateral surface over a
  sorted finite parameter interval, with a constant cylinder angle.

Radius and height must exceed ten absolute linear tolerances. Inputs and all
intermediate results must remain finite. Local radial closest approach avoids
forming a cancellation-prone squared discriminant. Direction normalization and
parameter conversion handle very small/large finite direction components;
unrepresentable parameters or merged roots fail explicitly. World-to-local
reconstruction and distinct generator end levels reject dimensions lost at
large world-coordinate offsets.

The radial gap uses the length policy at the cylinder-radius scale. Nonzero
gaps within that budget return an unresolved-contact error, rather than turning
a near secant/miss into a tangent or a near generator into an overlap. Exactly
zero computed gaps produce tangent/generator results, with checked surface and
line reconstruction. Nonzero radial direction within the angular budget is
unsupported. Rigidly transformed tangent/generator data can consequently be
rejected if rounding prevents an unambiguous classification; this API does not
silently snap geometry to manufacture an exact intersection.

Axial membership is closed, with no tolerance expansion or clamping. A nonzero
end-level separation within a floating calculation allowance of
`64 * epsilon * max(height, abs(anchor_z), abs(axial_travel))` is unresolved and
returns an error. Surface/radial residuals are checked before discarding roots
outside the axial interval, so numerically broken roots cannot become false
successful empty results.

These are checked f64 calculations, not certified exact 3D predicates. Neither
API applies a face's polygon, hole, arc, or partial-cylinder angular trims.
[Transverse planar trim clipping](face-intersections.md) now works separately.
Broader plane/cylinder curves, cylinder/cylinder intersections, NURBS
intersections, intersection graphs, curved-face splitting and general sewing
remain future work. [Scoped line/arc face subdivision](face-split.md) now works
with shared planar boundaries and rectangular cylinder-wall refinement. The earlier `cylinder_plane` horizontal Z-cylinder helper retains
its existing restricted contract.

## Runnable native/WASM fixture and evidence

```sh
cargo run --locked --example intersections           # rotated secants + UV
cargo run --locked --example intersections -- 0 2 0  # tangent
cargo run --locked --example intersections -- 1 0 0  # coincident planes
cargo run --locked --example intersections -- 2 2 0  # generator interval
./scripts/build-web.sh
node scripts/test-wasm.mjs
```

`hagane_generate_intersections(mode, offset, placement_angle)` exports the same
JSON fixture in WASM. It shares the existing output buffer/error contract.
Native tests cover ordered UV pcurves, opposite normals, angular/distance
policy, rigid placement, oblique axial clipping, end disks excluded, tangent
and generator cases, direction scales 1e-300/1e300, microscopic cylinders,
near contacts, invalid inputs and unrepresentable geometry. Runtime WASM tests
compare the full output with native results and independently verify analytic
roots, classifications, UV ranges, error reporting and recovery.
