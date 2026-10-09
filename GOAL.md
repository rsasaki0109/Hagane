# Development goal

**Complete Hagane as a usable pure Rust CAD kernel for an end-to-end CAD workflow.**

The workflow is: create or import exact geometry, construct and edit a valid
B-rep solid, inspect it interactively in a browser, and export it with its
geometry, topology, and units preserved. Deliver working implementations,
regression tests, examples, and documentation, not only designs or evaluators.
Broad OCCT-equivalent functionality remains the long-term direction; milestone
completion must never be presented as full OCCT compatibility.

## Active delivery target: approximately 80% of the long-term goal

The user has set the next standing target to approximately **80% completion**
of the long-term kernel goal. Continue autonomously toward this target when
asked to develop. This is an engineering estimate, not a count of presets,
commits or completed small features. Do not claim 80% from the current scoped
box/polygon-and-bore workflow. The latest estimate remains 15–25%.

Reaching the target requires working, documented and tested useful domains for
all of these major capability areas:

- NURBS curves/surfaces integrated with exact B-rep boundaries and validated
  display tessellation, beyond standalone evaluators.
- Boolean union, difference and intersection on a useful range of trimmed
  planar and curved solids, with explicit contact/conditioning failures.
- Practical fillet/chamfer operations and shape diagnostics/repair within
  documented domains.
- Useful STEP geometry/topology import and export, preserving units, with
  supported-subset round-trip regression tests.
- An editable native/WASM browser workflow connecting modeling and interchange,
  with reproducible documents and meaningful robustness/performance evidence.

The completion criteria below remain authoritative. Update the estimated
range and its remaining major gaps in final progress reports; do not assign
artificial percentage increments to individual milestones. Full OCCT parity,
all degenerate cases and assembly coverage remain longer-term work even after
this target. Implementations, tests, examples and verified limitations must
support any reassessment.

## User-value direction

Build measurable value for developers embedding exact modeling in web apps and
users generating parametric part variants. Prioritize explainable failures,
reproducible editable modeling intent and browser access alongside mathematical
coverage. Pure Rust/WASM and feature counts are not evidence of superiority.
See [product direction](docs/product-direction.md) for target outcomes, evaluation
and the next acceptance milestone.

The first scoped editable box-and-bore workflow now implements operation
history, structured diagnostics and versioned document round trips. Multiple
disjoint mixed through/blind machining nodes can be added, selected, edited and
removed while preserving remaining cuts. Incremental sessions now reuse
unchanged exact B-rep prefixes and rebuild only changed suffixes. Rejected
geometry or display edits retain the accepted cache; tests verify real operation
evaluation counts and equality to fresh rebuilds. Browser Undo/Redo now restores
accepted document edits, IDs and bore selection through the same validated
WASM session; rejected edits do not overwrite accepted history. Local autosave
restores validated current documents after reload; invalid saved data, quota
failures and competing tabs are handled without replacing valid geometry.
Editable polygon extrusion now creates exact stock from simple world-XY
profiles, including concave boundaries and disjoint polygon profile openings,
followed by disjoint mixed blind/through
bores. Profile/height edits, incremental suffix rebuilding, Undo/Redo and saved
document restoration share the same Rust kernel. Optional XY offsets now
create skew polygon stock with openings and disjoint Z-axis through/blind bores
in the same editable history. Swept-footprint clearance checks over each tool
depth reject side/opening crossings. Shallow blind holes outside the lower
profile retain exact walls/floors; unresolved floors and breakthrough are
explicitly rejected. Blind nodes now select top or bottom entry, preserving
exact floors, tool-depth clearance and saved editing intent. Opposing blind
bores may now overlap in XY when a resolved axial web separates their actual
cut intervals; touching/intersecting tools are explicitly rejected.
Reuse the existing exact supported operations; do not claim a general CAD
workflow from this limited first document format. This user-value milestone
runs ahead of further isolated primitive presets, while the long-term kernel
completion criteria below remain in force.

Planar-subject/convex-tool Boolean APIs now extend difference/intersection to
concave and polygon-holed subjects. Deterministic connected-partition scheduling
allows repeated cuts while preserving exact B-rep boundaries, checked volume
and a single connected shell. Native/WASM and browser demonstrations are
verified. This remains a scoped planar operation, not general curved Boolean
coverage, and is not yet a node in the editable operation document.

Multi-component planar plane partition now returns independently validated
closed solids per side, with exact shared section boundaries and conserved
combined volume. Native/WASM and browser side-selection demos verify a concave
stock separating into multiple parts. New planar-subject/convex-tool component Boolean APIs now preserve all closed
results through multi-part clipping and retained-boundary sewing. Native/WASM
and browser checks cover disconnected differences/common regions and conserved
volume. Original single-result APIs retain their documented limits. General
curved Booleans and editable Boolean document nodes remain subsequent work.

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
contacts and invalid inputs. Configurations without a
certificate remain unsupported. See [ellipse separation](docs/ellipse-separation.md).

Different Y-axis bore inclinations now support analytic full-height separation
certificates. A fixed projection direction with guarded same-sign endpoint gaps
proves that the tools never meet inside the plate; internally crossing axes are
rejected even when both caps are disjoint. Exact B-rep, analytic volume, small
dimensions, native/WASM parity and browser preset 27 are verified. Intersecting
tools and uncertified cases remain unsupported. See
[independently tilted bores](docs/divergent-tilted-bores.md).

Independent bore inclination and azimuth now preserve exact rotated ellipse
cap boundaries, oriented shared edges and cylindrical wall pcurves. Full-height
separation includes both XY drift components and rotated support radii; native
and WASM tests cover volume, roots, tiny geometry and invalid/crossing tools.
Browser preset 28 exposes the working closed solid. Intersecting or uncertified
tools remain unsupported. See [oriented bores](docs/oriented-bores.md).

Top-entry Z-axis cylindrical blind cuts now retain exact circular floors,
inward walls, cap hole wires and shared oriented edges in a closed B-rep.
Independent depths/radii, analytic volume, material above/below floors, tiny
and rotated solids, contact/breakthrough rejection and native/WASM parity are
verified; browser preset 29 displays the operation. Tilted/intersecting blind
cuts remain unsupported. See [blind bores](docs/blind-bore.md).

Blind cylindrical box cuts now enter any of the six selected faces, preserving
exact inward walls, outward floor normals, cap wires and shared topology through
right-handed rigid axis permutations. All-face tests verify volume, bounds,
classification, tolerance bands, microscopic/rotated solids, mesh closure and
wrong-face/breakthrough rejection. Native/WASM parity and side-entry browser
preset 30 are verified. Mixed-face, intersecting and oblique blind tools remain
unsupported. See [six-face blind bores](docs/box-face-blind-bore.md).

The first [editable workflow](docs/editable-workflow.md) rebuilds a centered box
with up to 256 mixed disjoint through/blind bores from schema-versioned
millimetre JSON.
Typed operation IDs, measured side/floor/pair diagnostics, rejected-tool outlines,
clearly labeled previous valid results, correction and save/load round trips
are verified natively, in WASM and in browser tests. Added nodes can be selected,
edited and removed with reference relinking; earlier edits preserve later cuts.
This remains a scoped linear box-and-bore history, not general parametric CAD
or persistent topology naming. Incremental sessions reuse unchanged validated
B-rep prefixes with explicit rebuilt-node counts, atomic failure recovery and
native/WASM parity. Other operation types, display-cache optimization and
comparative user-value measurements remain next work.

## Initial planar STEP export

[Exact planar STEP export](docs/step-export.md) now preserves analytic lines and
planes, shared oriented topology, polygon openings and explicit mm units in one
validated AP214 solid. Native/WASM byte parity and browser downloads are tested;
an optional independent reader verifies bounds, face count and signed volume.
The cylindrical extension follows below; general curved export, import and
assemblies remain unsupported. This advances interchange without completing
the long-term STEP goal; the overall engineering estimate remains 15–25%, toward the 80% target.

## Cylindrical STEP export

[Exact STEP export](docs/step-export.md) now extends the planar subset to complete circles
and rectangular full cylindrical faces. Shared periodic seams explicitly carry
both UV parameter curves; native cylinder/tube primitives, rigid placements and
through/blind bore workflows export exact geometry with mm units. Native/WASM
byte parity, browser downloads and independent external import checks verify
face counts, bounds, retained floors/webs and bounded mesh volume. Arcs, ellipses,
height-graph trims, skew circular surfaces and assemblies remain unsupported.
The initial convex planar reader follows below. The long-term estimate remains 15–25%, toward the 80% target.

## Initial convex planar STEP import

[STEP import](docs/step-import.md) now reads one closed convex planar straight-edge AP214
solid, preserves shared entity identities and separate edge/bound/face senses,
rebuilds affine pcurves and converts source SI mm/metres to mm. Closure, finite
positive volume and convex supporting-plane certification precede success.
An independently hand-authored metre tetrahedron, microscopic/rotated/large
round trips, bounded malformed inputs and native/WASM parity are tested. A
browser import/inspect/re-export page preserves previous valid results on
failure. An optional external reader checks source and native re-export.
The polygon-prism extension follows below. Curved and uncertified nonconvex
imports, assemblies and imported modeling history remain unsupported; the long-term estimate remains 15–25%, toward the 80% target.

## Certified polygon-prism STEP import

The planar reader now additionally accepts concave and polygon-holed straight
prisms, including skew/reversed extrusion and rigid placement. A bounded
translation certificate matches both cap rings, shared vertical links and every
oriented side quadrilateral without snapping or regenerating imported geometry.
Strict convex-only import remains available separately. Native tests cover
three scales, two openings, orientation, bounds ordering, empty hole material
and nearly parallel nonprismatic rejection. Native/WASM report parity and the
browser's actual skew plate import/re-export sample are verified. Curved and
uncertified nonconvex imports remain unsupported; see the STEP import document.
The long-term estimate remains 15–25%, toward the 80% target.

## Complete circular STEP import

The extended reader now imports complete cylinders and concentric tubes with
aligned circle/plane/cylinder frames and explicit periodic seam pcurves. Both UV
seams are checked in radians/converted length units. A separate geometric
certificate verifies coaxial cap/wall correspondence and inward tube walls,
preserving source geometry and shared topology. Strict planar-only APIs remain
available. Independent hand-authored metre geometry, three-scale/rigid round
trips, empty tube interior, chord errors and near-inconsistent seam/radius/height
rejection are verified natively; WASM report/byte parity, browser rotated-tube
import/re-export/recovery and independent external reading are tested. Partial
arcs and differently parameterized cylinders remain unsupported. Normal bores
in planar prism stock are supported by the subsequent milestone below. The long-term estimate remains 15–25%, toward
the 80% target. See the STEP import document for the bounded domain.

## Through-bored polygon STEP import

The reader now imports straight polygon stock with disjoint normal cylindrical
through bores, including concavity, polygon openings, skew extrusion and rigid
placement. A new public certificate verifies exact stock/tool boundary subsets,
shared cap ownership and complete swept-footprint clearance, retaining the
original imported B-rep. Native tests cover multiple tools, three scales,
reordered/reversed shells, analytic metrics/material and near contacts. A closed local B-rep whose tool crosses a moving opening only
at mid-depth is explicitly rejected. Native/WASM parity, real browser import/
re-export/recovery and independent external reading are verified. Oblique cuts, other mixed curved inputs and general Boolean results
remain unsupported imports; blind/opposing cuts are supported by the next
milestone. The long-term estimate remains 15–25%, toward the 80% target;
see the STEP import document for the precise supported domain.

## Normal blind and opposing STEP import

Polygon-prism import now preserves complete circular blind floors and accepts
normal entry from either cap, mixed through/blind tools, and opposing cuts with
a resolved axial web. Proof-only stock/tool subsets retain the source geometry;
entry owners select the stock axis, floor orientation is checked, and physical
cut intervals limit the swept stock/opening clearance test. Parallel tools are
separated by their radial/axial distance bound, allowing overlapping XY only
when their cut volumes remain separated. Floor/pair arithmetic guards reject
sub-roundoff webs and contacts rather than repairing the shape.
Native tests verify three scales, rigid/reordered geometry, exact volume/material
in the retained web and shallow cuts outside the lower profile, plus mid-depth
opening/tool intersections and unresolved floors/webs. Native/WASM report/byte
parity, browser import/download/upload/recovery and independent external retained
floor checks accompany the actual opposing-blind STEP fixture and screenshot.
Oblique/intersecting bores, arbitrary floors, internal cavities, subdivided stock
and general curved Boolean results remain unsupported imports. The long-term
estimate remains 15–25%, toward the 80% target; see the STEP import document.


## Large-placement numerical precision

Curved endpoints now validate in local coordinates before adding their world
origin, preventing a rounded-away radius from passing native rigid placement or
STEP seam reconstruction. Tessellation exposes a coordinate arithmetic allowance,
including cancelling parameter origins and circular height-band/skew coefficients,
reserves it from chord error and explicitly rejects unresolved display requests.
Closed mesh volume uses recentered tetrahedra with compensated summation.
Native regressions check collapsed circles, translated/reversed mesh volume,
far-cylinder chord error, cancelling UV origins and explicit local-frame recovery.
Native/WASM report/position/byte parity and independent WASM chord checks pass;
the actual far-coordinate STEP browser upload/orbit/download and rejected-input
recovery are tested and recorded. Existing external STEP regressions pass.
This strengthens the existing analytic domain without adding general curved
Booleans, NURBS B-rep integration or arbitrary numerical conditioning support.
The long-term estimate remains 15–25%, toward the 80% target. See
[coordinate precision](docs/coordinate-precision.md).


## NURBS curve refinement and bounded display milestone

Standalone positive-weight clamped NURBS curves now support shape-preserving
interior knot insertion, rational Bezier span extraction and adaptive display
polylines with original parameters and per-segment chord-error bounds. The
positive rational Bernstein convex hull supplies the mathematical geometric
bound; a conservative `f64` arithmetic allowance and explicit conditioning
errors handle the implementation's numerical limits. This is not formal
interval arithmetic certification. Refinement work, final controls, subdivision
depth and segment counts are bounded, with explicit failure rather than partial
success. Tests include degree 16, C0 limits, non-unit and large-origin parameter
domains, subnormal dimensions and independent quarter-circle chord distances.
The Rust/WASM browser demo now uses adaptive sampling, with an actual capture.

NURBS B-rep edges/pcurves, trimmed surfaces, intersections, general Booleans and
broader STEP interchange remain incomplete. This prerequisite does not establish
those capabilities. The overall estimate remains **15–25%**, toward the **80%**
target. See [NURBS refinement](docs/nurbs-refinement.md).


## NURBS tensor refinement and oriented boundary milestone

Standalone surfaces now support knot insertion on either axis using one common
homogeneous control-net weight scale, preserving shape and first partials.
Fixed-axis contraction returns exact rational isocurves with the other axis's
original knots and parameter domain. Rectangular boundaries expose four exact
curves, same-parameter affine UV maps and explicit counterclockwise traversal;
this is geometry data, not yet shared B-rep topology. Tests include independent
rational bilinear formulas, degree 16, C0/non-unit domains, large common weight
scaling, tiny dimensions, oriented closure and explicit resource failures.
The native/WASM surface demo now displays four boundaries and two selected
sections using bounded curve sampling after a shape-preserving 4-by-4 refinement.

The surface display grid remains uniform and has no certified surface chord
error. General trims, shared B-rep NURBS edges/faces, sewing, intersections and
broader STEP interchange remain incomplete. The overall estimate remains
**15–25%**, toward the **80%** target. See [surface refinement](docs/nurbs-surface-refinement.md).
