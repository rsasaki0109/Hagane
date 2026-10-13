# Torus line and ray intersections

`NurbsTorusSolid::intersect_line(origin, direction, policy)` returns every
resolved crossing of an infinite line with a regular ring torus.
`intersect_ray` retains its forward part. Both return `NurbsTorusLineIntersection`
with ordered actual face/UV hits and up to two material intervals. Parameters
refer to the original `origin + direction * t`; direction need not be unit
length. A ray uses `t >= 0`, including an interval starting at zero when its
origin lies inside material.

```rust
use hagane::{Frame3, GeometryTolerance, NurbsTorusSolid, Point3, Vec3};

fn main() -> hagane::Result<()> {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.0)?;
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16.0, 4.0], policy)?;
    let line = body.intersect_line(Point3::new(0.0, 0.0, 0.0),
                                   Vec3::new(2.0, 0.0, 0.0), policy)?;
    assert_eq!(line.hits.len(), 4);
    assert_eq!(line.material_intervals.len(), 2);
    let ray = body.intersect_ray(Point3::new(-16.0, 0.0, 0.0),
                                 Vec3::new(2.0, 0.0, 0.0), policy)?;
    assert_eq!(ray.material_intervals.len(), 2);
    assert_eq!(ray.material_intervals[0][0], 0.0);
    Ok(())
}
```

A checked local enclosure bounds the finite body by `±(R+r)` in X/Y and `±r`
in Z, expanded with the full original boundary band and arithmetic/frame
reserves. The original anchor is checked before any slab miss shortcut;
prospective unit-speed travel bounds directional arithmetic. Both cropped
endpoints must classify Outside against the actual torus. The existing
[finite segment solver](nurbs-torus-segment.md) then isolates all crossings.
No display mesh supplies the enclosure or intersections.

Mapping back to the original nonunit parameter checks world reconstruction
against the cropped segment and reevaluates the actual NURBS face witness.
Mapped hits and intervals must remain strictly ordered. Root isolation retains
the finite solver's absolute physical bracket contract; parameter mapping adds
its own checked arithmetic residual rather than claiming an exact polynomial
root or interval certificate.

Ray admission classifies the original origin and checks whether exactly one
material interval straddles zero when it is Inside. Forward intervals are
clipped to zero without dropping a second interval across the hole. Physical
travel checks the sign of each original parameter before retaining forward
hits. An infinite line may start on a transverse boundary; a ray origin in the
boundary band is explicitly unsupported.

Nonfinite/zero or unresolvable directions, remote anchors, insufficient world
or frame precision, tangencies, close roots, unresolved crop/mapping and
boundary-band ray origins return explicit errors. Rays currently resolve the
entire supporting line first: an unresolved backward tangency also refuses the
ray even if its forward part would miss. General torus closest points, trimming,
General STEP import and Booleans remain unsupported.

`NurbsTorusDemoSession::intersect_line` and `intersect_ray` read the actual
accepted Rust body and its tolerance policy. Separate six-value WASM transports
accept origin XYZ followed by original direction XYZ. Failed buffers and queries
preserve the accepted model and the independent edit/point/segment/other query
buffers. Reports contain `query_kind`, the original origin and direction, hits,
material intervals and `mesh_free: true`, without replacement mesh or STEP.

Run `cargo run --example nurbs_torus_line` or `nurbs_torus_ray`. Each example
also accepts a fourteen-value JSON array: the eight model parameters followed
by origin XYZ and direction XYZ. The browser demo has separate line and ray
buttons with the same world coordinates and nonunit direction inputs; dirty
model fields do not replace the accepted query source.


[Bounded cardinal torus STEP import](nurbs-torus-step-import.md) now retains
the actual canonical rational B-rep in its separate millimetre representation domain.
