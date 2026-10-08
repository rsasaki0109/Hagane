# Tolerance policy and exact planar predicates

Hagane distinguishes model-space distances, angles, relative scales, and exact
combinatorial signs. Exactness here refers to the finite binary64 input values;
it cannot recover precision lost before the call.

## Tolerance policy

`GeometryTolerance::new(linear, angular, relative)` validates and stores:

- Absolute length: finite and positive, in the caller's consistent model unit.
- Angular tolerance: radians, strictly between zero and pi/2.
- Relative tolerance: dimensionless, in [0, 1); zero disables relative scaling.

Defaults are `1e-8`, `1e-10`, and `1e-10`, respectively.
`length_at_scale(scale)` returns `max(linear, relative * scale)` for a finite
nonnegative **geometric length**. Use local dimensions or separations, rather
than the absolute world-coordinate offset. Point coincidence accepts an
explicit scale. Parallel/antiparallel and perpendicular tests normalize vectors
and compare the cross-product norm or absolute dot product to `sin(angular)`.
Zero/nonfinite directions are errors; normalization first scales components to
handle subnormal and large finite directions without squared-length overflow.

The existing `Tolerance { linear }` remains the absolute-distance contract for
primitive construction, extrusion, through-bores, topology, and tessellation.
`GeometryTolerance::try_from(tolerance)` preserves that distance and supplies
default angular/relative budgets. It validates publicly mutable legacy input.
The new relative policy is **not automatically applied to every CAD operation**.

Frame and plane basis checks use angular/relative budgets, not length units.
Rigid basis acceptance is capped at 1e-10; norm checks have a 64-epsilon floor
for floating arithmetic. `Transform::new` uses default angular/relative
budgets regardless of its legacy linear argument. `new_with_tolerance` permits
explicit tighter angle budgets. Accepted frame roundoff is orthonormalized;
scale, shear, and reflections remain outside the modeling transform API.

## Line/plane classification

`intersect_line_plane(anchor, direction, plane, policy)` returns:

- `Point { point, parameter }`, with `point = anchor + direction * parameter`.
- `Parallel`: direction lies within the angular budget of the plane, while the
  anchor lies outside its distance budget.
- `Coincident`: both direction and anchor lie within the corresponding budgets.

The latter two are classifications **within tolerance**, not proofs of exact
mathematical parallelism/coincidence. Distance scale is the anchor-to-plane-origin
separation. Near-parallel inputs can be treated more strictly by selecting a
smaller angular tolerance. The algorithm works with a normalized direction,
checks finite coordinate/parameter ranges, and checks line and plane residuals
against `max(linear, relative * max(separation, travel))`. Unrepresentable
parameters, overflow, and failed agreement return errors. Relative tolerance can
be disabled for an absolute-only distance policy.

The point-only `line_plane` compatibility wrapper uses the default policy and
returns `Unsupported` for both near-parallel classifications. It no longer uses
machine epsilon as its implicit angular threshold. The cylinder/plane routine
retains its documented axis-aligned restriction.

## Exact predicates

`orient2d(a, b, c)` returns the exact clockwise/collinear/counterclockwise sign of
`(b-a) × (c-a)`, including finite subnormal and very large coordinates. Nonfinite
inputs return `InvalidInput`. No tolerance or snapping is applied.

The fast path evaluates the determinant and accepts its sign only outside a
conservative roundoff bound of `8 * epsilon * (abs(left) + abs(right))`. Overflow,
underflow, and uncertain signs use a bounded integer fallback. Every finite
binary64 coordinate is decoded as a signed integer in units of 2^-1074.
Differences, products, and determinant subtraction then use original
little-endian base-2^32 magnitude arithmetic. The largest determinant needs
fewer than 132 limbs. This independent Rust implementation has no new library
dependency and does not incorporate reference implementation source.

For example, with `n = 134217728`, the determinant of `[0,0]`, `[n+1,n]`,
`[n,n-1]` is -1. Ordinary floating arithmetic rounds both products to the same
value and reports zero. `orient2d` returns clockwise.

`segments_intersect2d` uses these signs and inclusive coordinate bounds to
handle proper crossings, endpoint contact, collinear overlap, and degenerate
point segments. `locate_point_in_polygon` distinguishes `Inside`, `Boundary`,
and `Outside` using exact signs and a half-open even-odd crossing rule without
computing approximate ray intersections. Rings have 3..4096 corners; this
classification is not a ring simplicity validator. Self-crossing rings obey the
even-odd rule.

Polygon extrusion and B-rep planar trim validation now use these predicates
for segment crossings and containment. Near-contact clearance and redundant
corner rejection still use the existing **absolute metric tolerance**.
Segment projection uses normalized directions instead of squared lengths and
rejects nonfinite calculations explicitly. Exact sign checks do not certify
rounded distances, polygon areas, volume integration, intersections, circle
relations, or tessellation. General 3D predicates and robust face intersections
remain future work.

## Run and verify

```sh
cargo run --locked --example predicates -- 10 0.1
cargo run --locked --example predicates -- 10 1e-12
cargo run --locked --example predicates -- 1e-12 1e-12
cargo test --locked --test predicates
./scripts/build-web.sh
node scripts/test-wasm.mjs
```

The example emits the recovered determinant sign, scaled distance budget, and
line/plane classification (point, parallel, or coincident for the cases above).
WASM exports `hagane_orient2d` (-1/0/+1; 2 invalid),
`hagane_segments_intersect2d` (0 disjoint, 1 intersecting, 2 invalid), and the
shared-JSON `hagane_generate_predicates` fixture. Direct predicate calls do not
change the output buffer; generation calls follow the existing buffer lifetime.

Native tests use independent i128 determinants for 10,000 integer triples,
plus cancellation, full exponent range, boundary/contact, tolerance, extrusion,
and intersection cases. WASM tests compare 3,399 orientation cases and 204
segment cases against an independent JavaScript BigInt oracle, including broad
binary64 bit patterns, overflow/underflow, near collinearity and duplicate points.
The numerical fixture also checks native/WASM policy/classification parity.
