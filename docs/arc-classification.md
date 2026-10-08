# Bounded arc and partial-cylinder solid queries

![Actual WASM query on the concave circular notch](arc-classification.png)

`classify_point_in_solid` now accepts planar wires containing bounded lines and
circular arcs, including holes, and cylindrical faces with rectangular angular
and axial trims. These are analytic B-rep queries; tessellation only displays them.

Run `./scripts/build-web.sh`, serve `web/`, and open `classification.html`.
Select **Rounded plate** or **Arc notch and rounded hole**, orbit the part, and
move the query sliders. The same fixtures run natively:

```sh
cargo run --locked --example curved_classification -- 3 39 29 0
cargo run --locked --example curved_classification -- 4 0 16 0
```

The first query is outside the rounded corner; the second is on the notch wall.

Planar parity uses half-open line/arc crossings and retries unresolved endpoint
levels. Boundary distance selects the closest point on the finite arc, including
its endpoints. Angular membership for this distance uses the actual sweep rather
than a fixed angular slack. A partial wall combines the nearest finite circular
arc distance with axial distance using `hypot`, preserving a Euclidean rim band.
Ray intersections outside the angular trim are discarded; crossings within the
length budget of a trim generator retry another direction. Full-periodic walls
retain seam-independent crossings. Two resolved rays must agree.

Native tests compare rounded solids and their rigid placements with an analytic
grid, compare cylinder membership before/after periodic face subdivision, and
check arc-rim distances at ordinary and microscopic scales. WASM tests compare
native query results and B-rep meshes; browser tests exercise six models.

The existing closed/oriented validator runs before queries. General nonrectangular
cylinder trims, NURBS faces and geometric self-intersection detection remain
unsupported. Ambiguous rays may return an explicit error. This does not extend
the domain of general solid Boolean operations.

The implementation is original MIT OR Apache-2.0 Rust and adds no dependencies.
References: Euclidean distance to a finite circular arc, quadratic line/cylinder
intersection, and the even–odd crossing rule (Franklin, *PNPOLY*,
https://wrfranklin.org/Research/Short_Notes/pnpoly.html). No OCCT code is used.
