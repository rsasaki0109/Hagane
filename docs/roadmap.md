# Roadmap

## Working milestone

Analytic box/cylinder primitives, rectangle/disk extrusion, explicitly scoped
through-bore difference, shared oriented B-rep topology and pcurves, validation,
exact metrics, bounded display tessellation, and native/WASM interactive demo.

## Next: broaden analytic B-rep operations

- Explicit frames for curves/surfaces, solid transforms, and arbitrary extrusion.
- Structured angular/relative tolerances and robust filtered predicates.
- General planar wires, multiple holes, and non-complete cylindrical trims.
- Typed intersection results, curve/surface classification, intersection graph
  construction, face splitting, consistent sewing, and Boolean dispatch.
- Native/WASM property tests, difficult intersection corpora, and benchmarks.
- A stable browser API and versioned serialization.

## Later: full CAD kernel work

- B-splines and rational NURBS; adaptive evaluation, derivatives, intersections.
- General Boolean operations on arbitrary manifold solids.
- Fillets/chamfers and continuity constraints.
- Shape healing, tolerant sewing, and imported-shape diagnostics.
- STEP input/output with units, assemblies, and faithful geometry mapping.

These are plans, not stubbed operations or claims of current support. Every
added operation must define its input domain, error contracts, invariant checks,
reference provenance, native tests, and WASM compatibility before being listed
as implemented. OCCT source copying or translation is outside this project.
