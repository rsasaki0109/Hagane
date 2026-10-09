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

Repeated bounded-arc crossings now use descending original parameters to
refine shared rims/walls without losing subsequent events. Annular cap cuts,
reversed/placed and microscopic cases preserve topology, volume and the mesh
sagitta bound; an eleventh native/WASM/browser preset demonstrates this.

Periodic circle rims now refine into exact bounded arcs and cylinder wall
rectangles with a relocated seam that clears cut events. Disk/tube caps and
multiple full-circle bores, including original seam passage, preserve geometry,
volume and closed mesh seams. A twelfth browser fixture cuts a periodic bore.

Scoped exact planar sewing now reconstructs one closed manifold solid from
independent straight-boundary patches. Exact coincidences and exact collinear
subdivisions share vertices/edges/pcurves; near coincidences fail explicitly.
A thirteenth native/WASM/browser preset exercises an unmatched face subdivision.
See [sewing](docs/sewing.md).

Solid point classification now supports validated planar straight-edge B-reps,
including concavity, holes, subdivisions, skew extrusion and rigid placement.
Euclidean boundary distance uses a local tolerance budget; two independent
nondegenerate rays must agree. A native/WASM browser probe demo exercises exact
inside/outside/boundary queries. See [classification](docs/classification.md).

Scoped solid plane partition now constructs shared intersection vertices,
traces face/cap arrangements and sews negative/positive closed solids with
conserved analytic volume. Planar concave/hollow/skew and rigidly placed parts
are covered when both results are connected and the plane clears input vertices.
A fourteenth browser preset shows the positive part; native/WASM fixtures
verify both. See [solid partition](docs/solid-split.md).

Scoped convex solid intersection now clips a planar straight-edge operand by
checked outward supporting planes of another convex solid. Typed empty and
strict containment results are distinguished; transverse cuts create closed
B-reps. Analytic box and independently computed rotated-diamond volumes,
placement and microscopic cases are verified. A fifteenth native/WASM/browser
preset exercises two solid operands. See [convex intersection](docs/convex-intersection.md).

Scoped convex operand difference now selects original exterior fragments and
reversed cutter boundary faces, then sews one closed result. Nonconvex partial
cuts and rectangular through-holes, typed empty/unchanged results, volume
conservation, placement and tiny dimensions are verified. Internal generated
arrangements reconcile only bounded arithmetic roundoff; standalone sewing
stays strict. A sixteenth browser preset exercises a true two-solid subtraction.
See [convex difference](docs/convex-difference.md).

Scoped convex operand union now selects both operands' exterior fragments and
sews one closed boundary with checked combined volume. Containment, operand order,
nonconvex output, placement and tiny dimensions are verified. A seventeenth
native/WASM/browser preset demonstrates overlapping boxes. See
[convex union](docs/convex-union.md).

Axis-aligned box arrangements now support union/difference/intersection with
exact coplanarity and full/partial face contact. Strict sewing removes interior
interfaces and produces one closed shell. Identical operands, all contact axes,
regularized empty intersections and tiny/translated cases are verified; near
contacts and nonmanifold/disconnected/cavity results fail. An eighteenth
native/WASM/browser fixture demonstrates partial-face fusion. See
[box arrangements](docs/box-booleans.md).

Scoped coplanar face merging now removes interior face boundaries and reconstructs
outer/hole wires on certified identical supports. Geometry, shared topology,
original UV trims, volume, bounds and mesh closure are verified for contact and
through-hole parts, rigid placement and tiny dimensions. The nineteenth
native/WASM/browser preset reduces the contact fixture from 26 to 10 faces. See
[face merging](docs/face-merge.md).

Plane support identity now uses exact dyadic scalar triple products, enabling
coplanar face merging across independent tilted UV frames and origins. Affine
pcurves undergo checked local frame conversion; near supports remain separate.
Exact 3D orientation covers full finite binary64 inputs with independent native/
WASM integer reference tests. A twentieth browser fixture merges tilted union
faces. See [reframed merging](docs/reframed-merge.md).

Shared straight edge simplification now removes globally eligible degree-two
knots certified collinear in 3D and both incident UV boundaries. Original chain
endpoints, features, holes, face count, volume and closure are preserved. Native
tests cover minimal box topology, branches, placement, tiny dimensions and
idempotence. A twenty-first native/WASM/browser fixture reduces the merged
contact part from 34 to 24 shared edges. See
[edge simplification](docs/edge-simplify.md).

Analytic solid point classification now supports planar full-circle trims and
full periodic cylinder walls, including cylinders, tubes and multiple through-bores.
Euclidean cap/rim/wall bands, seam-independent ray crossings and two-ray agreement
are verified with 4950 analytic grid samples, placement, tiny dimensions and
unsupported-input checks. The browser query page now offers four exact solids;
native/WASM parity covers their probes and meshes. See
[curved classification](docs/curved-classification.md).

The next development task is broader retained-face selection and contact handling
using planar arrangements. Contact graphs,
arbitrary curved-face splitting, general sewing and NURBS B-rep integration
remain incomplete; the latter is stage 2 work.

Bounded planar line/arc trims and rectangular partial cylinder walls now support
analytic solid classification. Rounded material grids, rigid placement, microscopic
geometry, Euclidean arc-rim bands and periodic subdivision invariance are tested.
The native/WASM/browser query demo includes rounded plates and concave arc notches
with rounded through-holes. General cylinder trims and geometric self-intersection
detection remain unsupported. See [arc classification](docs/arc-classification.md).

Framed mixed-profile APIs now extrude line/arc regions with holes on arbitrary
rigid planes along either normal direction. Skew vectors beyond frame-conversion
roundoff fail explicitly. Volume, bounds, pcurves, cap endpoints, closed meshes,
arc sagitta, microscopic geometry and native/WASM parity are verified. The
twenty-second browser solid fixture demonstrates tilted negative-normal extrusion.
See [framed arc extrusion](docs/framed-arc-extrusion.md).

Mixed-profile skew extrusion now succeeds with exact circular translation
surfaces, retaining arcs, generators and surface parameter boundaries. Both
normal signs, curved holes, placement, analytic volume/bounds, UV/normals,
microscopic geometry and closed sagitta-bounded meshes are verified natively
and in WASM. A twenty-third browser fixture varies tangential offset. Classification and intersections are now implemented separately below;
general curved modeling of these skew walls remains unsupported.
See [skew extrusion](docs/skew-arc-extrusion.md).

Typed line/skew circular translation surface intersection now supports finite
points, tangent contact, and forward/reverse generator intervals. A conservative
shear-scaled cylinder reduction preserves original line parameters; recovered
world and local dimensions are checked. Native/WASM tests cover independent
roots, placement, tiny dimensions, conditioning and error recovery. The browser
contact-study page uses the exact skew circular B-rep fixture. Solid classification
and angular face clipping are implemented separately below. See
[extrusion intersections](docs/extrusion-intersections.md).

Rectangular circular B-rep face intersections now clip supporting hits to
angular trims and preserve original boundary edge parameters, periodic seam
uses and oriented normals. Exact dyadic incidence certifies shared-edge
generators and crossings; near-boundary inputs without certificates fail.
Native/WASM tests cover half/quarter faces, rims/corners, placement, tiny
geometry and world-coordinate precision loss. Browser face selection displays
actual open face meshes from the closed parent solid. General trims remain
unsupported; skew-solid classification is implemented separately below. See
[circular face intersections](docs/circular-face-intersections.md).

Skew circular solid classification now uses checked Euclidean boundary-band
bounds and analytic oriented rays on rectangular translation walls. Native
tests cover inverse-sheared material grids with curved holes, both signs,
placement, normal offsets, combined rim distance, tiny geometry and precision
rejection. Native/WASM queries and meshes agree; the seventh browser query
model is a skew plate with a rounded hole. General trims, self-intersection
detection, arbitrary curved-face splitting and general curved Booleans remain unsupported.
See [skew classification](docs/skew-classification.md).

Bounded skew circular face subdivision now preserves the physical translation
when rebasing child angular frames. The direct generator API refines both rims
and cap wires; planar cap splits also refine skew neighbors, including repeated
arc crossings and curved holes. Native checks verify child UV/normals, signed
placed/tiny solids, volume/bounds, membership and closed sagitta-bounded meshes.
Native/WASM/browser fixtures share the twenty-fourth solid preset. Full-periodic
skew rim refinement, arbitrary wall cuts and general curved Booleans remain
unsupported. See [skew subdivision](docs/skew-face-subdivision.md).

Oblique transverse plane boundary subdivision now uses exact bounded ellipse
section edges and harmonic wall pcurves. Shared generator splits propagate to
planar neighbors; analytical trim extrema, face flux and conforming shared-edge
meshes preserve a closed solid and its volume. Signed/placed/tiny cases, plane
incidence, loops, bounds, chord error, conditioning and failures are verified.
Native/WASM meshes and contours agree; the twenty-fifth browser solid preset
varies plane tilt. Rim contacts/partial crossings, full-periodic rims, repeated
harmonic-band cuts and capped solid partitions remain unsupported.
See [oblique boundary subdivision](docs/oblique-boundary.md).


Harmonic circular face line intersections now clip supporting roots and bounded
generator intervals against analytic height graphs. Exact dyadic ellipse-plane
incidence retains shared edge parameters; uncertified near-boundary inputs fail.
Native tests cover tangency, corners, reversal, tiny skew geometry and placement.
Native/WASM fixtures and browser band selection share the actual subdivided
B-rep. Arbitrary trim loops, repeated band cuts
and general curved Booleans remain unsupported. See
[harmonic face queries](docs/harmonic-face-intersections.md).

Harmonic circular wall solid classification now uses Euclidean ruled-trapezoid
distance bounds and analytic height-clipped rays. Shared ellipse edges count
as exterior boundaries; ambiguous ray contacts are retried rather than counted
twice. Native checks verify independent material/hole membership, signed/placed/
tiny solids, section edges, caps, normal bands, invalid topology and an unresolved
tolerance threshold. Native/WASM query and mesh parity and all eight browser
classification models are verified. General trim loops, repeated band subdivision,
capped partitions and general curved Booleans remain unsupported. See
[harmonic classification](docs/harmonic-classification.md).

Full-ellipse planar B-rep trims now retain shared angular pcurves, validated
whole-arc coefficient agreement and exact area/volume. Distinct near-coincident
half conics are rejected rather than treated as one ellipse. In-plane line clipping
and planar-face intersections return original line/ellipse parameters; solid
classification and conforming tessellation accept the closed ellipse-cap fixture.
Native tests cover roots, reversal, polygon-face intersection, tilt signs,
placement, microscopic dimensions, nonorthogonal axes, conditioning, malformed
inputs and contact rejection. Native/WASM queries and meshes and browser cap
selection/error recovery are verified. The initial domain was one complete two-half-arc
ellipse wire. The extensions below support specific mixed loops and concentric ellipse holes;
arbitrary ellipse-face subdivision and general capped partitions remain unsupported. See
[ellipse planar trims](docs/ellipse-planar.md).

Diameter-closed half-ellipse planar trims now support a single exact pi-sweep
ellipse arc followed by its straight diameter. In-plane queries retain original
line and ellipse parameters; boundary distance checks, solid classification,
analytic volume and conforming tessellation cover the mixed boundary. A closed
half-cylinder fixture retains material below an oblique plane. Native/WASM and
browser tests cover both crossings, reversal, placement, microscopic dimensions,
contact rejection and recovery. This domain is extended below; arbitrary mixed ellipse
loops, holes in arc/chord regions and general capped partitions remain unsupported. See [half-ellipse trims](docs/half-ellipse-planar.md).

Minor ellipse arcs closed by straight chords now extend the mixed trim domain
to resolved sweeps in `(0, pi]`. Analytic clipping preserves original line,
ellipse and chord parameters; physical boundary checks, classification, area,
volume and conforming tessellation use the actual arc interval. A symmetric
circular-segment extrusion with an oblique exact cap provides a closed fixture.
Independent bounds, roots, volume and display-chord error, rotated/tiny solids,
nonorthogonal axes, near contacts and unresolved thickness are tested.
Native/WASM parity and a 90-degree browser demo are verified. Larger sweeps, arbitrary composite loops and general capped partitions remain
unsupported. The ellipse-hole extension follows below. See [minor ellipse/chord trims](docs/ellipse-segment-planar.md).

Concentric homothetic full-ellipse holes now support exact annular planar caps.
Checked axis/center agreement and a physical clearance lower bound reject
unsupported or unresolved holes. Analytic clipping preserves outer/inner wire
provenance and returns two material intervals through the hole; polygon-face
intersections share those intervals. Classification, signed analytic area/volume
and conforming tessellation retain the hole. A closed obliquely capped tube
fixture, native/WASM example and actual browser demo are implemented. Tests cover
independent roots/volume/bounds, rotated/tiny solids, nonorthogonal axes, physical
bands, shared orientation, display-chord error and no triangles filling the hole.
The offset- and multiple-hole extensions follow below. Holes in arc/chord loops,
arbitrary mixed loops and general capped partitions remain unsupported. See
[ellipse annuli](docs/ellipse-annulus-planar.md).

Offset homothetic full-ellipse holes now extend annular cap support beyond a
common center. Inverse outer-axis coordinates and a singular-value clearance
bound certify strict containment; touching or unresolved offsets are rejected.
Whole-curve 3D/pcurve coefficient agreement is checked at arithmetic precision
as well as model tolerance. Original displaced roots, wire provenance, one/two
material intervals, classification and conforming meshes preserve the actual
hole. The closed eccentric-tube fixture retains the correct first-moment term
in oblique-cut volume. Native checks cover XY offsets, tilt signs, rotation,
tiny dimensions, independent volume/roots, nonorthogonal trims, containment,
corrupt pcurves and oriented meshes with no filled hole. Native/WASM parity and
an actual browser demo are verified. The multiple-hole extension follows below;
arbitrary mixed loops and general capped partitions remain unsupported. See
[offset ellipse holes](docs/ellipse-eccentric-planar.md).

Multiple disjoint aligned homothetic ellipse holes now extend full-ellipse
planar trims to at most sixteen holes. Inverse outer-axis circle coordinates and
physical clearance lower bounds validate containment and pairwise separation;
overlap, nesting, touching, near touching and excessive counts are rejected.
Sorted shared-parameter events preserve original wire/edge provenance and emit
all material intervals independently of input order or line direction.
Classification, analytic first-moment volume and conforming tessellation retain
every hole. Native tests cover three-hole roots/volume/material, reordered and
reversed queries, rotated/tiny solids, nonorthogonal trims, all-wall chord errors,
closed meshes without filled holes, four polygon/cap intersection segments and
the sixteen-hole limit. Native/WASM parity and a two-hole browser demo are verified.
Holes in arc/chord loops, arbitrary mixed loops and general
capped partitions remain unsupported. See [multiple ellipse holes](docs/ellipse-multi-hole-planar.md).

Unequal-axis complete ellipse holes now support conservative normalized-circle
containment and separation certificates. A closed circular plate with a tilted
true cylindrical through bore preserves exact ellipse rims, shared edges and
harmonic cylinder pcurves. Native/WASM and browser checks cover analytic volume,
classification, bounded tessellation and invalid inputs. Main solid preset 25
and a cap intersection demo expose the implementation. Arc/chord holes and
general curved Boolean operations remain unsupported.
See [tilted through bores](docs/tilted-bore.md).

Analytic supporting-line certificates now accept separated ellipse holes even
when enclosing circles overlap. Translated parallel tilted cylindrical bores
form closed solids with exact ellipse rims and harmonic pcurves; preset 26
exposes radius controls. Native/WASM and browser tests cover volume, material,
contacts and invalid inputs. Nonparallel bores and configurations without a
certificate remain unsupported. See [ellipse separation](docs/ellipse-separation.md).
