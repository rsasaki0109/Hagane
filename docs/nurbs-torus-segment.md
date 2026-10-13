# Finite torus segment intersections

`NurbsTorusSolid::intersect_segment(start, end, policy)` queries a validated
regular ring torus without using its display mesh. It returns ordered hits and
material intervals in the original segment parameter `start + t * (end-start)`,
with `0 <= t <= 1`. A segment can cross four boundaries and contain two separate
material intervals across the central hole. Inside endpoints clip intervals to
zero or one. Each hit includes the actual retained NURBS face, UV, evaluated
boundary point, residual and outward normal. Cardinal seams within the
arithmetic allowance select a deterministic actual quarter start; the final
measured surface residual must still pass the boundary policy.

```rust
use hagane::{Frame3, GeometryTolerance, NurbsTorusSolid, Point3};

fn main() -> hagane::Result<()> {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.0)?;
    let body = NurbsTorusSolid::new(Frame3::IDENTITY, [16.0, 4.0], policy)?;
    let result = body.intersect_segment(
        Point3::new(-24.0, 0.0, 0.0),
        Point3::new(24.0, 0.0, 0.0), policy)?;
    assert_eq!(result.hits.len(), 4);
    assert_eq!(result.material_intervals.len(), 2);
    Ok(())
}
```

The implementation independently derives the quartic implicit equation
`(x²+y²+z²+R²-r²)² - 4R²(x²+y²) = 0`. Its sign agrees with signed tube
distance for a regular ring torus. Scaled coefficients and guarded cubic
Bernstein subdivision isolate regions where its derivative has definite sign.
Unresolved derivative regions become collars: the one-Lipschitz tube distance
must prove the entire collar clear of the boundary. Opposite guarded endpoint
signs in the remaining monotone regions isolate individual crossings.

Root brackets have physical width at most the original absolute linear
tolerance divided by sixteen. Actual surface evaluation supplies each witness;
normal direction checks entering/exiting behavior. A supplemental forward-error
guard accounts for shallow crossing angles and the smaller of the tube radius
and inner ring clearance. The full original body's relative tolerance band
controls contact and separation; a query does not enlarge that policy.
These are conservative binary64 engineering guards, not interval arithmetic.

Tangencies, boundary-band endpoints, insufficient separation, shallow or
unresolved crossings, zero/tiny segments, nonfinite coordinates, insufficient
world/frame precision and exhausted isolation budgets return explicit errors.
The bounded solver uses at most 4096 subdivision cells and depth 64; its
bisection budget is 128 iterations. A rejected query leaves the B-rep unchanged.
[Line/ray queries](nurbs-torus-line.md) use checked finite enclosures.
General trimming, general torus STEP import and torus Booleans remain
unsupported.

`NurbsTorusDemoSession::intersect_segment` reads the actual accepted body and
accepted tolerance policy, even when model fields contain unaccepted edits.
Its small JSON report contains endpoints, hits and material intervals without
replacing the accepted mesh or STEP. The separate six-value WASM transport
accepts start XYZ followed by end XYZ; malformed buffers preserve the model
and the independent point-query/edit buffers.

Run `cargo run --example nurbs_torus_segment` for the four-crossing example.
It also accepts a JSON array of fourteen values: the eight model parameters
from `nurbs_torus`, followed by start XYZ and end XYZ. Browser instructions are
in [the torus demo](nurbs-torus.md).


[Bounded cardinal torus STEP import](nurbs-torus-step-import.md) now retains
the actual canonical rational B-rep in its separate millimetre representation domain.
