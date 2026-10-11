# Mesh-free torus point queries

`NurbsTorusSolid::point_query(point, policy)` classifies a world point against
the validated regular ring body. `classify_point` returns only the location.
Both methods use the actual typed B-rep and never query its display mesh.

For local cylindrical radius `rho`, the ideal signed boundary distance is
`hypot(rho - R, z) - r`. The original body's complete size determines the
relative tolerance band; query distance does not enlarge that band. World
coordinate and represented-frame arithmetic are checked before inside/outside
shortcuts. The central hole and the tube's center circle do not require a
division by zero.

Boundary results contain an actual face index, UV, evaluated surface point,
Euclidean distance and outward normal. Major and minor angles are mapped to
the corresponding rational quarter parameters, then the retained NURBS face is
evaluated. A measured distance with its arithmetic reserve must fit the caller's
boundary band. Unresolved threshold comparisons and insufficient query precision
return explicit errors. These are binary64 engineering guards, not an interval
certificate or a general closest-point solver.

```rust
use hagane::{Frame3, GeometryTolerance, NurbsTorusSolid, Point3, PointLocation};

fn main() -> hagane::Result<()> {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.0)?;
    let torus = NurbsTorusSolid::new(Frame3::IDENTITY, [16.0, 4.0], policy)?;
    assert_eq!(torus.classify_point(Point3::new(0.0, 0.0, 0.0), policy)?,
               PointLocation::Outside);
    let result = torus.point_query(Point3::new(20.0, 0.0, 0.0), policy)?;
    assert_eq!(result.location, PointLocation::Boundary);
    println!("{:?}", result.boundary);
    Ok(())
}
```

`NurbsTorusDemoSession` retains the last successfully serialized model.
`apply` constructs and validates its geometry, display and STEP report before
replacing the accepted body. `query_point` reads that retained Rust solid and
its accepted tolerance policy. Rejected model edits, failed queries and dirty
browser fields cannot change the query's source geometry. The consumer retains
the accepted mesh and STEP; a point query returns neither replacement.

Run `cargo run --example nurbs_torus_query` for the default tube-center query.
The example also accepts a JSON array of eleven values: the eight model values
used by `nurbs_torus`, followed by world point XYZ. The browser probe is on
`nurbs-torus.html`; see [the torus demo instructions](nurbs-torus.md).

This supports point classification and boundary witnesses on regular ring tori.
Line/ray/segment intersections, general closest points, trimming, torus STEP
import and torus Booleans remain unsupported.
