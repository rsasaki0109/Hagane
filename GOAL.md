# Development goal

**Complete Hagane as a usable pure Rust CAD kernel for an end-to-end CAD workflow.**

The workflow is: create or import exact geometry, construct and edit a valid
B-rep solid, inspect it interactively in a browser, and export it with its
geometry, topology, and units preserved. Deliver working implementations,
regression tests, examples, and documentation, not only designs or evaluators.
Broad OCCT-equivalent functionality remains the long-term direction; milestone
completion must never be presented as full OCCT compatibility.

## Completion criteria

- Exact curves and surfaces, including NURBS, integrate with shared, oriented
  B-rep edges and faces, surface parameter boundaries, shells, and solids.
- Primitives, arbitrary-plane profiles, extrusion, and rigid transforms form
  valid solids with explicit units and tolerance contracts.
- Curve/surface intersections, face splitting, classification, and sewing
  support solid union, difference, and intersection beyond the current
  box/through-cylinder special case.
- Fillets, chamfers, and shape diagnostics/repair work within clearly stated
  supported domains. Unsupported cases return useful errors.
- STEP import and export support a documented useful subset, preserve units
  and exact geometry/topology, and reject unsupported entities explicitly.
- B-rep-derived tessellation respects a documented approximation bound,
  including trimmed NURBS faces. Display meshes never substitute for modeling.
- The same kernel works natively and in WASM. A browser demo exercises a real
  multi-step modeling workflow with orbit/zoom and import/export.
- Tests cover analytic dimensions/volumes, closure and orientation, shared
  edges and pcurves, round trips, contacts, near coincidences, small dimensions,
  singularities, invalid inputs, and unsupported operations.
- CI checks formatting, strict clippy, native tests, WASM builds/runtime parity,
  and browser behavior. English documentation, actual demo recordings, examples,
  licenses, dependency notices, and algorithm references accompany delivery.

Each feature must state its supported domain and evidence before it counts as
complete. A limited first implementation is acceptable; silently approximating
an exact modeling operation, returning an unchanged shape as success, or
claiming general support from a special case is not.

## Execution order

1. **Geometry foundations:** common frames and solid transforms, arbitrary-plane
   profiles, angular/relative tolerances, robust predicates, and mixed line/arc
   wires. Extend NURBS refinement and derivatives as needed by subsequent work.
2. **Exact surface topology:** integrate NURBS with B-rep, support surface trims,
   and produce tessellation with checked approximation bounds.
3. **General Boolean infrastructure:** typed intersections, classification,
   intersection graphs, face splitting, and sewing; then union, difference,
   and intersection with progressively broader supported inputs.
4. **Modeling and repair:** fillets/chamfers, diagnostics, tolerant sewing,
   and explicitly scoped healing.
5. **Interchange:** STEP geometry/topology mapping, units, import/export,
   supported-subset round trips, and assembly support as a later expansion.
6. **Release readiness:** stable documented APIs, versioned serialization,
   a complete browser workflow, regression corpora, performance benchmarks,
   installation instructions, and recorded demos.

Validation and documentation accompany every stage, rather than waiting until
stage 6. Reorder prerequisites when implementation evidence warrants it; keep
this goal and the roadmap synchronized.

## Current baseline

Implemented: analytic box/cylinder/tube solids, scoped multiple through-bores,
polygon extrusion with holes and skew/reversed directions, oriented B-rep and
pcurve validation, exact metrics, bounded analytic display tessellation, and
native/WASM browser demos. Standalone NURBS curves and tensor-product surfaces
support evaluation and first derivatives with dedicated demos.

Still incomplete: NURBS B-rep integration, trimmed NURBS tessellation, general
Booleans, fillets/chamfers, healing, and STEP interchange. NURBS surface display
grids currently have no certified chord-error bound. See
[the roadmap](docs/roadmap.md) and [README](README.md) for feature-specific limits.

Stage 1 now includes checked coordinate frames, rigid analytic B-rep transforms,
exact placed bounds, and arbitrary-plane polygon extrusion, with native/WASM
validation and a browser preset. See [frames](docs/frames.md).

Stage 1 also includes explicit length/angular/relative policies, classified
line/plane intersection and filtered exact 2D predicates integrated with planar
validation. See [tolerances](docs/tolerances.md) for scope and verified cases.

Stage 1 now includes simple convex/concave line/arc regions with sharp joins,
signed circular arcs, multiple curved/polygon holes and either input winding.
Positive normal extrusion retains exact edges, pcurves and cylinder walls;
analytic classification and trim separation protect construction. Checked
sampled trims and conforming cap triangles protect display. A seventh solid
preset exercises the concave arc-notch and rounded hole in native/WASM/browser.
See [mixed profiles](docs/mixed-profiles.md).

Typed plane/plane intersections now provide unit-speed lines and affine UV
curves on both planes. Line/Z-or-framed-cylinder intersections provide sorted
hits, parameters, UV, tangency and bounded generator overlap, with checked
contact/axial ambiguity errors and native/WASM examples. These foundational APIs operate on
surfaces; separate planar face clipping now applies supported trims. See [intersections](docs/intersections.md).

Transverse line clipping now preserves analytic planar boundaries, original
edge parameters and interior intervals, including concavity and curved holes.
Two planar faces produce finite intersection segments with shared normalized
pcurves, verified natively and in WASM. Contact/vertex/overlap/coplanar cases
remain unsupported. See [face intersections](docs/face-intersections.md).

Scoped planar line/arc face subdivision now creates two selected-face children,
updates shared straight boundaries and refines bounded cylinder rims/walls.
Opposite caps receive matching subarcs; analytic hole ownership preserves curved
holes. Closed topology, pcurves, geometry, volume and mesh seams are verified
natively and in WASM, with the ninth browser preset. See
[face subdivision](docs/face-split.md).

Multi-interval planar cut graphs now split polygon/arc holes and concave caps
into connected child faces. Shared boundary refinement and analytic cycle/hole
ownership preserve closure, volume and conforming meshes. A tenth browser
preset cuts through two polygon holes, with native/WASM parity.

The next development task is repeated hits on one boundary edge and periodic
circular boundary subdivision. Contact graphs,
arbitrary curved-face splitting, general sewing and NURBS B-rep integration
remain incomplete; the latter is stage 2 work.
