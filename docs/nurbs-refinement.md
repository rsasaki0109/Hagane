# NURBS refinement and bounded curve display

Hagane's standalone `NurbsCurve` now supports interior knot insertion, rational
Bezier span extraction and adaptive display polylines. These operate on the
existing positive-weight, clamped, nonperiodic curve domain. They also supply
the exact boundaries of [rectangular open NURBS faces](nurbs-face.md); generic
NURBS solid operations and bounded surface tessellation remain unsupported.

![Actual Rust/WASM bounded NURBS curve demo](nurbs-bounded.png)

## API

```rust
use hagane::{NurbsCurve, Point3};
let curve = NurbsCurve::new(2,
    vec![2.0, 2.0, 2.0, 6.0, 6.0, 6.0],
    vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0),
         Point3::new(0.0, 1.0, 0.0)],
    vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0])?;
let refined = curve.insert_knot(4.0, 2)?;
let spans = refined.bezier_spans()?;
let polyline = refined.tessellate_bounded(0.001, 16_384)?;
assert_eq!(polyline.points.len(), polyline.parameters.len());
assert_eq!(polyline.error_bounds.len() + 1, polyline.points.len());
```

`insert_knot(parameter, times)` refines homogeneous controls using convex
interpolation. It preserves the mathematical curve and its one-sided derivatives;
computed controls are subject to `f64` rounding. Interior multiplicity cannot
exceed the degree. Endpoint insertion is unsupported, while zero insertions
return an unchanged clone after checking that the parameter lies in the closed
domain. There is no tolerance-based parameter snapping.

`bezier_spans()` raises each interior knot's multiplicity to the degree. Each
`RationalBezierSpan` contains `parameter_range`, `control_points` and `weights`;
`evaluate(parameter)` uses the original parameter interval, not a new 0..1
parameterization. Shared endpoints retain positional continuity, including C0
knots. Public span fields can be edited, so evaluation validates their domain,
cardinality, coordinates and positive weights before using them.

`NurbsPolyline` contains ordered `points`, original `parameters`, and one
`error_bounds` entry per segment. The bound applies to the curve interval between
the corresponding parameters and its straight chord. The polyline is a display
approximation; the original rational curve remains the modeling geometry.

## Mathematical bound and floating-point allowance

With positive weights, a rational Bernstein curve is a convex combination of
its Euclidean control points. Distance to a chord segment is a convex function,
so the maximum control-point distance to that segment bounds the curve's
curve-to-chord distance. Homogeneous de Casteljau subdivision tightens that
bound without changing the underlying curve. A zero-length chord is treated as
a point. Each output segment is accepted only when its bound, including the
arithmetic allowance, is no larger than the requested error.

The convex-hull statement is an exact mathematical result. The implementation
uses `f64`, not interval arithmetic or an exact arithmetic proof. It adds a
conservative engineering allowance

`4096 * f64::EPSILON * max(coordinate_scale, f64::MIN_POSITIVE)`
`* (maximum_weight / minimum_weight) * (degree + 1)`.

Here `coordinate_scale` is the maximum absolute source control coordinate.
The allowance accounts for coordinate placement, homogeneous refinement,
projection and subdivision; it is not a general formal certificate of every
floating-point operation. If it is nonfinite or consumes at least a quarter of
the requested error, the operation returns an explicit tessellation error.
Large placement coordinates, extreme weight ratios and extremely small error
requests can therefore be rejected even when a visually plausible polyline
could be drawn. Local differences and scaled direction normalization avoid
reciprocal overflow for subnormal chords; nonfinite projections cannot be hidden
by a maximum reduction.

A representable parameter midpoint can differ from the arithmetic half of an
interval. Subdivision uses `(middle - start) / (end - start)`, ensuring that its
geometry corresponds to the recorded parameter. Unsplittable parameter intervals
return an error when the existing chord cannot meet the requested bound.

## Resources and failures

- The existing degree 1..16 and at most 65,536 curve controls remain in force.
- Bezier extraction checks the final control count before refinement and limits
  cumulative refinement work to 16,000,000 estimated control updates.
- `max_segments` must be 1..1,000,000. The source's nonzero knot-span count is
  checked against it before Bezier extraction.
- Adaptive subdivision stops at depth 48. Exhausting depth, representable
  parameters or segment resources returns an error, never a partial success.
- Nonfinite error requests, nonpositive errors, invalid parameters and invalid
  public span data are rejected explicitly.

Error and coordinates use the caller's consistent length unit. Parameter and
knot units are independent; domains need not be 0..1.

## Native and browser demos

```sh
cargo run --locked --example nurbs_refinement
cargo run --locked --example nurbs_refinement -- 0.7071067811865476 0.5 0.0001
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Open the **NURBS curves** page from the solid demo. The browser uses
`hagane_generate_nurbs_bounded(weight, parameter, chord_error)` and displays the
requested error, achieved maximum bound and segment count. The points,
derivative, adaptive polyline and per-segment bounds come from Rust/WASM; the
canvas only renders them. The old `nurbs` CLI and `hagane_generate_nurbs` fixture
remain uniform-sample compatibility APIs and do not claim these bounds.

Tests cover quarter-circle geometry with independent dense chord-distance
samples, shape and derivative invariance, C0 knots, degree 16, common weight
scaling, non-unit and adjacent-float parameter domains, large parameter origins,
subnormal geometry, invalid public spans and resource failures. Browser and
native/WASM parity checks exercise the same adaptive demo path.

## Next steps

Extend rectangular open-face integration to cross-face shared edges, general
trimmed NURBS surfaces, curve/surface intersections and broader STEP interchange.
Periodic curves, higher derivatives and surface refinement remain future work.
See [references](references.md) for mathematical provenance; no external NURBS
implementation is linked or copied.
