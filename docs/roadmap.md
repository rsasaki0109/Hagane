# Roadmap

This roadmap implements the standing [development goal](../GOAL.md): a usable
end-to-end pure Rust CAD workflow. Feature-specific supported domains and
verified completion criteria take precedence over broad feature labels.

## Working milestone

Analytic box/cylinder/tube primitives; rectangle/disk extrusion; simple concave
or convex XY polygon extrusion with multiple polygon holes and skew/reversed
directions; explicitly scoped multiple through-bore difference; shared oriented
B-rep topology and pcurves; trim containment/separation and manifold validation;
exact metrics; bounded display tessellation; and four native/WASM interactive
presets with recorded demos. Standalone positive-weight clamped NURBS curves
now support position and analytic first-derivative evaluation, validated knots,
explicit one-sided limits, and an interactive native/WASM curve demo.
Standalone tensor-product NURBS surfaces now support points, analytic U/V
partials, regular-point normals, knot-side selection, and checked uniform
display grids, with a 3D native/WASM surface demo.

Checked immutable coordinate frames, rigid B-rep placement, framed circles and
cylinders, exact world-axis bounds, and arbitrary-plane polygon extrusion now
work natively and in WASM, with a fifth interactive solid preset. See
[frames](frames.md) for numerical and operation limits.

Structured length/angular/relative policies, classified line/plane intersections,
filtered exact 2D orientation, exact segment contact and polygon point location
now underpin planar validation. Native integer and WASM BigInt reference corpora
verify signs independently. Metric distances and 3D predicates remain floating
calculations; see [tolerances](tolerances.md).

## Next: broaden analytic B-rep operations

- Extend frame-aware primitive/profile APIs and intersection routines.
- Extend explicit tolerance policies to more modeling operations, and add exact
  3D predicates and robust curve/surface intersections.
- Mixed arc/line planar wires, non-complete cylindrical trims, and general face splits.
- Typed intersection results, curve/surface classification, intersection graph
  construction, face splitting, consistent sewing, and Boolean dispatch.
- Native/WASM property tests, difficult intersection corpora, and benchmarks.
- A stable browser API and versioned serialization.

## Later: full CAD kernel work

- NURBS knot refinement, higher derivatives, periodic curves, certified adaptive
  subdivision, higher/mixed surface derivatives, surface trimming, intersections,
  and integration with B-rep edges/faces.
- General Boolean operations on arbitrary manifold solids.
- Fillets/chamfers and continuity constraints.
- Shape healing, tolerant sewing, and imported-shape diagnostics.
- STEP input/output with units, assemblies, and faithful geometry mapping.

These are plans, not stubbed operations or claims of current support. Every
added operation must define its input domain, error contracts, invariant checks,
reference provenance, native tests, and WASM compatibility before being listed
as implemented. OCCT source copying or translation is outside this project.
