# Roadmap

This roadmap implements the standing [development goal](../GOAL.md): a usable
end-to-end pure Rust CAD workflow. Feature-specific supported domains and
verified completion criteria take precedence over broad feature labels.

The active delivery target is approximately 80% of the long-term goal, with
major capability acceptance areas recorded in [GOAL.md](../GOAL.md). The current
estimate remains 15–25%; narrow workflow milestones do not establish broad
CAD coverage.

## Immediate product milestone

Following the [product direction](product-direction.md), the next priority is
an explainable editable box-and-bore workflow: real operation history, parameter
editing, typed failure diagnostics, visible failed candidate tools and versioned
JSON operation-document round trips shared by native and WASM. A
[first scoped implementation](editable-workflow.md) now passes browser
failure/correction and round-trip tests, exact B-rep validation and independent
geometry checks for a box plus up to 256 disjoint mixed through/blind bores,
including selection, editing, deletion/relinking and pair-clearance diagnostics.
Incremental sessions now reuse unchanged validated B-rep prefixes and rebuild
changed suffixes, with native/WASM equality to fresh results, cache-preserving
failure recovery and browser-visible operation counts. Browser Undo/Redo
restores up to 64 accepted edits, including imports, dimensions and bore
add/remove operations; rejected edits preserve accepted history and redo.
Local autosave now restores current documents through Rust validation, preserves
rejected/corrupt data and detects conflicting tab writes; storage failures leave
modeling and JSON download usable. Polygon extrusion roots now integrate
profile/height edits and subsequent exact circular bores into the same history,
including concave boundary containment, incremental invalidation, Undo/Redo and
JSON/local restoration. Polygon profile openings now support up to 64 disjoint
loops and 256 total corners, exact subsequent bores, opening-clearance diagnostics
and complete editing/save/restore integration. Finite XY offsets now add skew
polygon stock to document history with exact caps/walls, profile openings,
editing and restoration. Disjoint world-Z through/blind bores now retain exact
cap circles/cylinder walls and blind floors on skew polygon stock. Swept-footprint
certificates use the actual tool depth, rejecting side/opening crossings and
near contacts; blind floor thickness is independently checked. Blind nodes now
select top/bottom entry with exact floor normals and depth-interval checks,
incremental editing and validated browser restoration. Opposing blind cuts
now accept overlapping XY footprints when their depth intervals retain a
resolved web, with measured axial/pair diagnostics and atomic rejection. Curved profiles and arbitrary-plane extrusion frames
remain outside the document scope. Other operation types,
mesh-cache optimization and persistent topology references remain planned.
Measure task success and rebuild behavior before claiming an advantage over
established CAD tools. Further isolated presets are secondary to this workflow.

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

Simple line/arc regions now support sharp joins, concavity, signed arcs,
multiple curved/polygon holes and either winding, with exact positive normal
extrusion. Analytic point classification, adjacent/nonadjacent intersection
checks, hole containment/separation, normalized oriented topology, and checked
conforming cap tessellation accompany a seventh native/WASM/browser solid
fixture. General curved surface trims remain unsupported; see
[mixed profiles](mixed-profiles.md).

Typed plane/plane intersection now returns lines with pcurves on both planes.
Line/Z-or-framed-cylinder intersection returns checked lateral hits, original
parameters, UV, tangency and axial generator intervals, with native/WASM
fixtures. Near contacts and unrepresentable calculations fail explicitly.
See [intersections](intersections.md).

Transverse planar face clipping now returns original-parameter boundary events
and material intervals for polygon, circle and line/arc trims with holes.
Planar-face pairs return finite segments and normalized pcurves on both faces.
Native/WASM fixtures verify analytic intervals, provenance and volume preservation.
Tangent/vertex/overlap/coplanar clipping cases remain unsupported;
see [face intersections](face-intersections.md).

Scoped planar line/arc face subdivision now splits straight boundaries and
bounded rims, refines rectangular cylinder walls along shared generators,
updates opposite caps and assigns curved holes analytically. Top/bottom,
repeated/reversed and inward-wall cuts preserve closed topology, volume and
mesh seams. A ninth native/WASM/browser solid fixture exercises curved cap
splitting. Multi-interval cut graphs now cross polygon/arc holes and create
multiple connected children. Repeated crossings on one bounded arc now refine
shared walls/rims using original-parameter order, with an eleventh annular-cap
native/WASM/browser fixture. Periodic full-circle rims now refine into exact
quarter arcs and rectangular walls with cut-safe relocated seams, demonstrated
by a twelfth periodic-bore preset; see
[face subdivision](face-split.md).

Exact planar patch sewing now merges identical vertices and shares straight
edges, propagating exact collinear subdivisions across independent face
boundaries. A thirteenth native/WASM/browser fixture reconstructs a box from
patches with an unmatched boundary subdivision. Closed, oriented manifold
validation rejects open/duplicated/disconnected shells; near coincidences are
not healed. See [sewing](sewing.md).

Planar straight-edge solid point classification now uses analytic plane/trim
intersections, Euclidean boundary bands, signed entry/exit checks and agreement
of two independent rays. Concavity, polygon holes, rigid/skew placement and
subdivided faces are covered, with a native/WASM browser query demo. Full periodic cylinders and planar circles are now supported separately below;
see [classification](classification.md).

Transverse solid plane partition now builds shared edge intersections and
closed section loops, keeps planar material on both sides, and sews two closed
solids with conserved volume. Concave/hollow/skew and placed parts are supported
when each result stays connected and the cut clears original vertices. Native/
WASM fixtures verify both parts; a fourteenth browser preset shows the positive
part. See [solid partition](solid-split.md).

Convex planar straight-edge solid intersection now clips by checked outward
supporting half-spaces, creates closed B-reps and returns typed empty results.
Strict containment, analytic box overlap, rotated-diamond area, placement and
small dimensions are verified, with a fifteenth native/WASM/browser preset.
Contacts/coplanar cases and nonconvex/curved operands remain unsupported. See
[convex intersection](convex-intersection.md).

Convex operand difference now discards artificial partition interfaces and
sews original exterior fragments plus reversed cutter boundary faces into one
closed shell. Nonconvex/through-hole outputs, empty/unchanged cases, placement
and tiny models are verified, with a sixteenth native/WASM/browser fixture.
Enclosed cavities, disconnected results and general contacts remain unsupported.
See [convex difference](convex-difference.md).

Convex operand union now retains both operands' exterior fragments and sews one
closed, potentially nonconvex boundary. Analytic combined volume, containment,
operand order, rigid placement and microscopic dimensions are verified, with a
seventeenth native/WASM/browser preset. Disjoint/contact/coplanar inputs remain
unsupported. See [convex union](convex-union.md).

Axis-aligned box arrangements now support all three regularized Booleans with
exact coplanarity and full/partial face contact. Strict coordinate sharing and
sewing remove internal interfaces; analytic volume and closed topology are
verified. Identical operands, all contact axes and tiny/translated cases are
covered by an eighteenth native/WASM/browser fixture. Near contacts and
nonmanifold/disconnected/cavity results fail. General convex contact handling
remains unsupported. See [box arrangements](box-booleans.md).

Scoped coplanar face merging now reconstructs outer/hole boundaries after removing
shared interior face edges. Certified identical frames or exact axis-aligned
supports preserve geometry, pcurves, volume, bounds and mesh closure. Rigid/tiny
parts and hole/disconnected-region cases are covered, with a nineteenth
native/WASM/browser preset reducing 26 faces to 10. Shared straight edge simplification is available separately; healing remains
future work.
See [face merging](face-merge.md).

Exact scalar triple products now certify plane identity across independent
origins and tilted UV bases. Checked affine pcurve conversion broadens face
merging without snapping near supports. Exact 3D orientation is independently
verified natively and in WASM across the full finite binary64 range. A twentieth
native/WASM/browser fixture exercises independent tilted frames. See
[reframed merging](reframed-merge.md).

Shared straight edge simplification now removes exact degree-two collinear
knots in 3D and both incident UV wires, preserving features, holes and geometry.
Minimal box topology, branches, classification, closed meshes, placed/tiny parts
and idempotence are verified. A twenty-first native/WASM/browser fixture reduces
34 shared edges to 24. Curved merging and tolerant straightening remain unsupported.
See [edge simplification](edge-simplify.md).

Analytic solid classification now supports planar full-circle wires and full
periodic cylinder walls. Cylinder/tube/multiple-bore membership, cap/rim/wall
Euclidean bands, periodic seams, placement and tiny dimensions are verified.
4950 independent analytic grid samples accompany native/WASM probes/mesh parity;
the browser query page offers eight solid models, including bounded arc trims, rectangular partial cylinder walls, skew circular translation walls and harmonic height bands. General cylinder and NURBS trims remain unsupported. See [curved classification](curved-classification.md).

Framed line/arc region extrusion now accepts either world-space normal sign on
arbitrary rigid planes, preserving exact walls, holes and shared pcurves.
Analytic volume/bounds, tiny inputs, signed/winding variants and closed meshes
are verified, with a twenty-second native/WASM/browser fixture. Skew arc
extrusion is implemented separately below. See [framed extrusion](framed-arc-extrusion.md).

Exact skew mixed-profile extrusion now constructs circular translation surfaces
with signed vectors, curved holes, arbitrary rigid placement and checked shared
pcurves. Analytic volume/bounds and sagitta-bounded closed meshes are verified.
The twenty-third native/WASM/browser fixture varies skew offset at fixed volume.
Solid classification is now implemented separately below; general curved modeling
operations on the new walls remain unsupported.
See [skew arc extrusion](skew-arc-extrusion.md).

Line/skew circular translation surface intersections now reduce analytically
to the cylinder solver with conservative shear-scaled guards and original-space
line/surface verification. Sorted finite hits, tangency and signed generator
intervals are checked natively/in WASM. A browser contact-study page exercises
secants, tangents, misses and overlaps with actual B-rep-derived geometry.
Solid classification and face angular clipping are implemented separately below. See
[extrusion intersections](extrusion-intersections.md).

Trimmed circular face intersections now select rectangular cylinder/skew-wall
angular domains, report original shared-edge parameters and oriented normals,
and retain bounded generator overlap. Exact dyadic line incidence certifies
boundary cases without tolerance snapping. Periodic seams, partial walls, tiny
geometry and error recovery are verified natively/in WASM. The browser contact
page displays either selected semicircular face. General trims remain unsupported;
skew-solid classification is implemented separately below. See [circular face intersections](circular-face-intersections.md).

Skew circular solid classification now supports rectangular translation walls
with Euclidean chord-patch distance bounds and independent oriented ray checks.
Signed, placed and microscopic solids with rounded holes are verified against
independent analytic grids. Native/WASM parity, error recovery and the seventh
browser classification model are tested. Unresolved bounds return explicit
errors; general trims and self-intersection detection remain unsupported.
See [skew classification](skew-classification.md).

Bounded skew circular face subdivision now supports generator cuts and rim
refinement alongside planar cap cuts. Rebased child frames transform drift to
preserve the exact world translation, shared opposite generators and cap
pcurves. Signed, placed, tiny, repeated and inward-hole cases are verified,
including closed meshes, membership and analytic volume/bounds. The twenty-fourth
solid preset has native/WASM parity and browser slider checks. Full-periodic
skew rims and arbitrary curved-wall cuts remain unsupported.
See [skew subdivision](skew-face-subdivision.md).

Oblique transverse plane boundary subdivision now creates exact ellipse arcs
and harmonic height pcurves, preserving closed shared topology and analytical
volume. Wall flux quadrature, signed/placed/tiny geometry, shared mesh sampling,
ellipse error and rejection of rim contacts/partial crossings are verified.
Native/WASM/browser contours and meshes share the twenty-fifth solid fixture.
Periodic rims, repeated band cuts and capped solid partitions remain unsupported.
See [oblique subdivision](oblique-boundary.md).

Harmonic circular face line intersections now clip supporting roots and bounded
generator intervals against analytic height graphs. Exact dyadic ellipse-plane
incidence retains shared edge parameters; uncertified near-boundary inputs fail.
Native tests cover tangency, corners, reversal, tiny skew geometry and placement.
Native/WASM fixtures and browser band selection share the actual subdivided
B-rep. Arbitrary trim loops, repeated band cuts
and general curved Booleans remain unsupported. See
[harmonic face queries](harmonic-face-intersections.md).

Harmonic circular wall solid classification now uses Euclidean ruled-trapezoid
distance bounds and analytic height-clipped rays. Shared ellipse edges count
as exterior boundaries; ambiguous ray contacts are retried rather than counted
twice. Native checks verify independent material/hole membership, signed/placed/
tiny solids, section edges, caps, normal bands, invalid topology and an unresolved
tolerance threshold. Native/WASM query and mesh parity and all eight browser
classification models are verified. General trim loops, repeated band subdivision,
capped partitions and general curved Booleans remain unsupported. See
[harmonic classification](harmonic-classification.md).

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
[ellipse planar trims](ellipse-planar.md).

Diameter-closed half-ellipse planar trims now support a single exact pi-sweep
ellipse arc followed by its straight diameter. In-plane queries retain original
line and ellipse parameters; boundary distance checks, solid classification,
analytic volume and conforming tessellation cover the mixed boundary. A closed
half-cylinder fixture retains material below an oblique plane. Native/WASM and
browser tests cover both crossings, reversal, placement, microscopic dimensions,
contact rejection and recovery. This domain is extended below; arbitrary mixed ellipse
loops, holes in arc/chord regions and general capped partitions remain unsupported. See [half-ellipse trims](half-ellipse-planar.md).

Minor ellipse arcs closed by straight chords now extend the mixed trim domain
to resolved sweeps in `(0, pi]`. Analytic clipping preserves original line,
ellipse and chord parameters; physical boundary checks, classification, area,
volume and conforming tessellation use the actual arc interval. A symmetric
circular-segment extrusion with an oblique exact cap provides a closed fixture.
Independent bounds, roots, volume and display-chord error, rotated/tiny solids,
nonorthogonal axes, near contacts and unresolved thickness are tested.
Native/WASM parity and a 90-degree browser demo are verified. Larger sweeps, arbitrary composite loops and general capped partitions remain
unsupported. The ellipse-hole extension follows below. See [minor ellipse/chord trims](ellipse-segment-planar.md).

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
[ellipse annuli](ellipse-annulus-planar.md).

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
[offset ellipse holes](ellipse-eccentric-planar.md).


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
capped partitions remain unsupported. See [multiple ellipse holes](ellipse-multi-hole-planar.md).

Planar-subject / convex-tool difference and intersection now accept concave or
polygon-holed subjects, including repeated straight-sided cuts. A deterministic
schedule defers only plane partitions that disconnect an intermediate shell;
closed retained faces, pcurves, volume conservation and native/WASM/browser
results are checked. Curved tools, cavities, disconnected outputs and editable
Boolean document nodes remain unsupported. See
[planar/convex Booleans](planar-convex-booleans.md).

Multi-component plane partition now preserves separate closed planar solids
on either side, with exact component grouping, no near-vertex snapping, checked
per-component topology and total volume. Native/WASM and browser checks cover
concave stock with two separated arms. The original one-solid partition API
retains its rejection of multiple pieces. New component difference/intersection
APIs now carry all pieces through convex-tool clipping and shared-edge sewing,
with explicit empty/unchanged results, conserved volume and native/WASM/browser
checks. General curved Booleans and document history integration remain next work. See
[multi-component partition](solid-split-components.md) and
[component Booleans](component-booleans.md).

## Next: broaden analytic B-rep operations

- Extend frame-aware primitive/profile APIs and intersection routines.
- Extend explicit tolerance policies to more modeling operations, and add exact
  3D predicates beyond orientation and robust curve/surface intersections.
- Extend ellipse planar trims to mixed line/arc/ellipse loops and holes.
- Extend oblique plane subdivision to partial/rim contacts and capped solid partitions.
- Extend general trim clipping and subdivision to skew circular translation
  surfaces; broaden classification beyond harmonic height bands and support arbitrary cylindrical
  trim loops and general face splits.
- Broaden retained-face selection and contact handling using
  planar arrangements; extend contact handling, surface/surface curves, intersection
  graphs and curved-face splits.
- Native/WASM property tests, difficult intersection corpora, and benchmarks.
- A stable browser API and versioned serialization.

## Later: full CAD kernel work

- Higher NURBS derivatives, periodic curves, surface patch splitting, bounded arbitrary trimmed
  surface tessellation, higher/mixed surface derivatives, surface trimming, intersections,
  and integration with B-rep edges/faces.
- General Boolean operations on arbitrary manifold solids.
- Fillets/chamfers and continuity constraints.
- Shape healing, tolerant sewing, and imported-shape diagnostics.
- Broader STEP curve/trim import, curved/NURBS export and assembly/unit mapping beyond
  the working single-solid planar/full-cylinder export subset.

These are plans, not stubbed operations or claims of current support. Every
added operation must define its input domain, error contracts, invariant checks,
reference provenance, native tests, and WASM compatibility before being listed
as implemented. OCCT source copying or translation is outside this project.

Unequal-axis complete ellipse holes now support conservative normalized-circle
containment and separation certificates. A closed circular plate with a tilted
true cylindrical through bore preserves exact ellipse rims, shared edges and
harmonic cylinder pcurves. Native/WASM and browser checks cover analytic volume,
classification, bounded tessellation and invalid inputs. Main solid preset 25
and a cap intersection demo expose the implementation. Arc/chord holes and
general curved Boolean operations remain unsupported.
See [tilted through bores](tilted-bore.md).

Analytic supporting-line certificates now accept separated ellipse holes even
when enclosing circles overlap. Translated parallel tilted cylindrical bores
form closed solids with exact ellipse rims and harmonic pcurves; preset 26
exposes radius controls. Native/WASM and browser tests cover volume, material,
contacts and invalid inputs. Configurations without a
certificate remain unsupported. See [ellipse separation](ellipse-separation.md).

Different Y-axis bore inclinations now support analytic full-height separation
certificates. A fixed projection direction with guarded same-sign endpoint gaps
proves that the tools never meet inside the plate; internally crossing axes are
rejected even when both caps are disjoint. Exact B-rep, analytic volume, small
dimensions, native/WASM parity and browser preset 27 are verified. Intersecting
tools and uncertified cases remain unsupported. See
[independently tilted bores](divergent-tilted-bores.md).

Independent bore inclination and azimuth now preserve exact rotated ellipse
cap boundaries, oriented shared edges and cylindrical wall pcurves. Full-height
separation includes both XY drift components and rotated support radii; native
and WASM tests cover volume, roots, tiny geometry and invalid/crossing tools.
Browser preset 28 exposes the working closed solid. Intersecting or uncertified
tools remain unsupported. See [oriented bores](oriented-bores.md).

Top-entry Z-axis cylindrical blind cuts now retain exact circular floors,
inward walls, cap hole wires and shared oriented edges in a closed B-rep.
Independent depths/radii, analytic volume, material above/below floors, tiny
and rotated solids, contact/breakthrough rejection and native/WASM parity are
verified; browser preset 29 displays the operation. Tilted/intersecting blind
cuts remain unsupported. See [blind bores](blind-bore.md).

Blind cylindrical box cuts now enter any of the six selected faces, preserving
exact inward walls, outward floor normals, cap wires and shared topology through
right-handed rigid axis permutations. All-face tests verify volume, bounds,
classification, tolerance bands, microscopic/rotated solids, mesh closure and
wrong-face/breakthrough rejection. Native/WASM parity and side-entry browser
preset 30 are verified. Mixed-face, intersecting and oblique blind tools remain
unsupported. See [six-face blind bores](box-face-blind-bore.md).

## Initial planar STEP export

[Exact planar STEP export](step-export.md) now preserves analytic lines and
planes, shared oriented topology, polygon openings and explicit mm units in one
validated AP214 solid. Native/WASM byte parity and browser downloads are tested;
an optional independent reader verifies bounds, face count and signed volume.
The cylindrical extension follows below; general curved export, import and
assemblies remain unsupported. This advances interchange without completing
the long-term STEP goal; the overall engineering estimate remains 15–25%, toward the 80% target.

## Cylindrical STEP export

[Exact STEP export](step-export.md) now extends the planar subset to complete circles
and rectangular full cylindrical faces. Shared periodic seams explicitly carry
both UV parameter curves; native cylinder/tube primitives, rigid placements and
through/blind bore workflows export exact geometry with mm units. Native/WASM
byte parity, browser downloads and independent external import checks verify
face counts, bounds, retained floors/webs and bounded mesh volume. Arcs, ellipses,
height-graph trims, skew circular surfaces and assemblies remain unsupported.
The initial convex planar reader follows below. The long-term estimate remains 15–25%, toward the 80% target.

## Initial convex planar STEP import

[STEP import](step-import.md) now reads one closed convex planar straight-edge AP214
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
[coordinate precision](coordinate-precision.md).


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
target. See [NURBS refinement](nurbs-refinement.md).


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
**15–25%**, toward the **80%** target. See [surface refinement](nurbs-surface-refinement.md).


## First retained rectangular NURBS B-rep face

`Curve::Nurbs` and `Surface::Nurbs` now participate in the common geometry enums,
with checked evaluation/normal dispatch and exact-control rigid placement.
`NurbsFace` retains a complete rectangular surface using the existing four
vertices, four edges, oriented coedges, affine same-parameter pcurves and Face.
Boundary validation checks shared corner references, canonical rational basis
and weights, control-coordinate tolerance and traversal; it does not establish
global surface regularity, injectivity, sewing or a closed manifold. Singular
structural faces are accepted and fail display sampling when normals are unusable.

The native/WASM browser now samples its retained open face; whole-surface grid
sampling remains uncertified. Generic NURBS Solid validation/volume/Boolean/
classification/STEP paths remain unsupported; arbitrary trims and cross-patch
shared edges remain future work. This supersedes the earlier standalone-only
NURBS boundary milestones without establishing general rational solid support.
The engineering estimate remains **15–25%**, toward the **80%** target.
See [rectangular NURBS face](nurbs-face.md).


## Bounded single-span rational surface display

`NurbsSurface::tessellate_bounded` and the retained `NurbsFace` wrapper now
provide conforming shared-vertex grids with original UV cell ranges and a
per-cell two-triangle error bound. Tensor Bernstein coefficients bound the
rational surface's difference from its bilinear corner interpolant; a twist
term bounds the latter's difference from the triangles. Original-evaluator
corner mismatch and a conservative floating-point allowance are added separately.
This mathematical geometric bound is not a formal interval arithmetic proof.

Only single positive rational Bezier patches are supported. Uniform dyadic
levels preserve conformity, while cell/depth/control-work and representable
parameter limits fail explicitly. Analytic corner normals and triangle checks
do not certify global regularity or injectivity. Multi-span/C0 surfaces,
arbitrary trims, adjacent-face stitching and generic NURBS solid operations
remain unsupported. Legacy sample_grid and surface fixture paths remain
uncertified; the native/WASM browser now uses the bounded face API and reports
requested/achieved error. Dense independent parameter and point-to-triangle
distance tests cover rational/warped/degree16/tiny/nonunit cases and failures.

The estimate remains **15–25%**, toward the **80%** target. See
[bounded surface display](nurbs-surface-tessellation.md).


## Exact rational patch extraction and C1 multi-span bounded display

`NurbsSurface::bezier_patches()` refines both tensor axes to Bezier multiplicity
with a common homogeneous control-net weight scale and returns original-domain
rational patches. Public extraction supports C0 boundaries and one-sided patch
partials; bounded display conservatively requires C1 from knot multiplicity,
even for smooth geometry represented with nominal C0 knots. Final control grid,
patch count and cumulative refinement work are preflighted before refinement.

The bounded surface API now uses all extracted nonuniform patches with one
global dyadic level, shared original UV vertex indices and original-source
analytic normals. Previous Bernstein/twist/corner error bounds remain; the
engineering arithmetic guard accounts for extraction separately from the exact
mathematical inequalities, without claiming formal interval proof. The source
span product is checked against max_cells before extraction. The actual retained
browser source can be refined from 3-by-3 to 4-by-4/four spans with shape preserved,
and exposes patch count, ranges and bounded display metrics.

C0 meshing, arbitrary trims, cross-face sharing/sewing, global regularity, closed
NURBS solids, intersections and STEP remain incomplete. Legacy single-span and
uniform compatibility APIs remain available. The estimate remains **15–25%**,
toward the **80%** target. See [surface extraction](nurbs-surface-extraction.md).


## Bounded C0 rational faces with one-sided crease normals

Bounded rational surface display now accepts C0 knot lines, including degree-one
multi-span ridges and smooth geometry represented at nominal C0 multiplicity.
Each incident cell chooses its own analytic one-sided normal, with all positions
reusing the original source's continuous UV evaluation. `vertex_uv`,
`vertex_nodes` and `normal_sides` separate geometric connectivity from shading
vertices: seam copies are bit-identical and share logical nodes, four side pairs
can meet at a crossing, and unrelated UVs are never welded solely by XYZ. Mixed
signed-zero knots canonicalize cache and crease keys without changing cell ranges.

The existing Bernstein geometric bounds, precision/resource guards and global
dyadic conformity remain. Ordinary source normal/partial queries at C0 still
require explicit sides. Singular side normals and sampled orientation errors
fail explicitly, without establishing global regularity or manifoldness. The
native/WASM browser now retains an actual 3-by-2 rational roof face and can show
either selected normal limit while keeping both in its bounded mesh. Smooth
fixtures and legacy APIs remain available. Arbitrary trims, cross-face sewing,
closed rational solids, intersections and STEP remain incomplete.

This supersedes earlier C1-only display restrictions. The estimate remains
**15–25%**, toward the **80%** target. See [C0 face display](nurbs-surface-crease.md).

## Exact rectangular NURBS restriction

Rational surfaces now restrict to an original-parameter UV rectangle by
homogeneous knot insertion, tensor control selection and endpoint clamping.
The retained open face regenerates four exact isocurve edges, shared corners,
oriented coedges and affine pcurves while preserving face orientation. C0 cut
endpoints use their inward derivative limits; retained interior creases preserve
split normals and shared geometric nodes. No fitting or mesh Boolean is used.

Native source/derivative and B-rep boundary tests, bounded display checks and
native/WASM/browser edit/rejection/recovery accompany the actual demo. Resource
preflight limits refinement controls and cumulative work. Mathematical shape
preservation is subject to checked `f64` rounding; display bounds concern the
retained restricted surface and remain engineering bounds, not interval proofs.

This is rectangular surface restriction, not arbitrary trim-loop support.
General trims/holes, cross-face sewing, closed NURBS solids, intersections and
NURBS STEP remain future work. The estimate remains **15–25%**, toward **80%**.
See [rectangular restriction](nurbs-surface-trim.md).

## Rectangular inner wires on rational faces

Exact rational isocurves and affine same-parameter maps now bound separated
rectangular UV openings in a retained open B-rep face. The outer wire traverses
counterclockwise and hole wires clockwise, retaining shared corner references.
Boundary coordinates become source knot lines, so bounded global tensor cells
conform to every opening. Cells inside the exact trim are omitted from display
while retained cells keep geometric error bounds and C0 normal-side metadata.

Native boundary/coverage/orientation tests, independent logical edge incidence
and rational error checks, native/WASM parity and actual browser editing/recovery
accompany the demo. Parameter clearance is separate from linear tolerance.
Contact, overlap, unresolved separation and resource demand fail explicitly.

This supports axis-aligned rectangular UV inner wires, not arbitrary trim curves
or closed NURBS solids. Cross-face sewing, global regularity diagnostics,
intersections, curved Booleans and NURBS STEP remain incomplete. The estimate
remains **15–25%**, toward **80%**. See [UV openings](nurbs-surface-hole.md).

## Material-only bounded rational face display

Exact excluded knot-span cells are now removed before computing approximation
bounds, subdivision and analytic normals. Retained cell budgets therefore do
not charge removed interiors, and a singular point strictly inside a hole no
longer fails display because the complete supporting surface was sampled first.
Global dyadic conformity, original UV nodes, one-sided normals and per-cell
geometric bounds remain unchanged. Full-net refinement/extraction and numerical
conditioning guards still apply.

Native singularity/budget regressions and independent native/WASM/browser checks
exercise a polynomial surface whose isolated singular point is excluded by an
exact inner wire. Retained sampled singularities and resource/accuracy failures
still return errors. No source repair, global regularity certificate, arbitrary
trim support or closed NURBS solid is claimed. The estimate remains **15–25%**,
toward **80%**. See [retained-domain display](nurbs-surface-material.md).

## Exact diagonal UV edges on rational surfaces

An affine UV path now composes with the homogeneous tensor Bernstein surface
to produce exact mathematical rational curve pieces at original knot crossings.
The retained B-rep edge has canonical endpoint references and an affine pcurve
using the same T=0..1 parameter. Constant-axis paths contract the corresponding
degree; reversed paths retain direction and correct derivative side semantics.
Degree, resource, endpoint and numerical conditioning limits fail explicitly.

Native independent formulas and chain-rule/side tests, bounded curve checks,
native/WASM parity and an actual browser endpoint editor accompany the demo.
This does not fit a curve from sampled points or claim arbitrary face trim
support. Surface regularity, general loops, sewing, closed NURBS solids,
intersections and interchange remain future work. The estimate remains
**15–25%**, toward **80%**. See [surface edges](nurbs-surface-edge.md).

## Closed convex UV surface wires

`NurbsSurfaceWire` now retains exact lifted edges, affine pcurves and shared
closing vertex indices for 3–64 strictly convex UV corners, in either traversal
direction. Validation rejects unsupported polygons and corrupted topology.
Native tests and an executable example verify rational edge/surface identity
and bounded boundary display. Face interior tessellation, arbitrary trim loops,
sewing and closed NURBS solids remain incomplete. The long-term estimate remains
15–25%, toward 80%. See [surface wires](nurbs-surface-wire.md).

## Convex UV polygon faces

`NurbsPolygonFace` retains an open face with an exact convex straight-UV outer
wire, shared boundary topology and an independent signed face orientation.
A scoped affine-patch interior tessellator and OBJ example verify analytic area
and normals. Curved face construction is supported, while curved interior display
returns an explicit unsupported error. General trims, holes, sewing and closed
rational solids remain incomplete. The estimate remains 15–25%, toward 80%.
See [polygon faces](nurbs-polygon-face.md).

## Bounded curved bilinear polygon display

Convex UV faces now have conforming bounded interior display for one equal-weight
bilinear patch, including genuinely curved saddles. Shared midpoint indices,
original UVs and per-triangle engineering bounds preserve the retained region.
Native independent formula/coverage/orientation tests and an actual OBJ demo
verify this scope. General rational/higher-degree/multi-span polygon display,
holes, sewing and closed rational solids remain incomplete. Native/WASM builds
share the kernel; no dedicated browser export was added. The estimate remains
15–25%, toward 80%. See [bounded polygon display](nurbs-polygon-bounded.md).

## Positive-weight rational bilinear polygon display

Bounded polygon tessellation now accepts nonuniform positive weights on one
bilinear patch. Homogeneous derivative bounds and a barycentric Taylor remainder
replace the polynomial-only twist bound, with explicit numerical-conditioning
and budget rejection. Independent rational formulas and common weight scaling
tests verify the actual implementation; the OBJ demo now uses nonuniform weights.
Higher-degree/multi-span display, holes, sewing and closed rational solids remain
incomplete. The estimate remains 15–25%, toward 80%. See [bounded polygon
display](nurbs-polygon-bounded.md).

## High-degree rational polygon display

`NurbsPolygonFace::tessellate_bounded` extends conforming polygon display to one
positive-weight high-degree Bezier patch. General Bernstein first/second derivative
control nets supply checked Hessian bounds. Independent biquadratic and asymmetric
cubic/linear rational formulas test every emitted triangle bound; a weighted
biquadratic OBJ demo exercises actual retained-face display. The bilinear API
retains its original contract. Multiple spans, general trim loops, holes, sewing
and closed rational solids remain incomplete. The estimate remains 15–25%,
toward 80%. See [Bezier polygon display](nurbs-polygon-bezier.md).

## C1 multi-span rational polygon display

Bounded polygon display now covers structurally C1 source knots through scaled
per-Bezier-patch derivative bounds. Independent Cox–de Boor evaluation verifies
triangles crossing both U and V knots in nonunit domains; the OBJ demo exports
an actual four-span rational face. Structural C0 knots are explicitly rejected,
including geometrically smooth over-multiplied knots. Full-source extraction,
derivative work and display budgets remain checked. C0 polygon interiors, holes,
general trims, sewing and closed rational solids remain incomplete. The estimate
remains 15–25%, toward 80%. See [C1 polygon display](nurbs-polygon-multispan.md).

## Crease-aware rational polygon display

`tessellate_crease_bounded` splits convex UV triangle domains at structural C0
knots, sharing intersection/midpoint identities and geometric positions while
retaining explicit one-sided normals. Independent piecewise rational bounds,
coverage/connectivity and analytic normals verify crossed creases and boundary
ownership. An OBJ demo exports actual geometry with separate normal indices.
Existing C1 APIs retain their rejection contract. Holes, general loops, sewing
and closed rational solids remain incomplete. The estimate remains 15–25%,
toward 80%. See [crease-aware polygon display](nurbs-polygon-crease.md).

## Editable browser rational polygon faces

The shared native/WASM polygon fixture now exposes an actual retained triangular
UV B-rep face and bounded interior display in three modes: weighted biquadratic,
C1 refinement and crossed C0 roof. Browser editing, exact-edge boundary overlays,
one-sided normals and rejected-edit preservation are verified alongside full
JSON parity and independent curve/surface/bound checks. A captured actual screen
documents the working UI. This is an open face, not a closed solid. General loops,
holes, sewing and closed rational solids remain incomplete. The estimate remains
15–25%, toward 80%. See [polygon demo](nurbs-polygon-demo.md).

## Rectangular inner wires on convex rational faces

`NurbsPolygonHoledFace` retains exact clockwise inner wires and shared global
references inside a convex UV outer boundary, with independently guarded UV
clearance and explicit contact/overlap rejection. Convex cell partition before
triangulation preserves material boundaries, C0 normal identities and excluded
singularity sampling. Native independent bounds/area/topology/resource tests,
full native/WASM diagnostic parity and an editable browser opening are verified.
Nonrectangular holes, general trims, sewing and closed rational solids remain
incomplete. The long-term estimate remains 15–25%, toward 80%. See [convex
face openings](nurbs-polygon-holes.md).

## First closed polynomial NURBS solid domain

The scoped `NurbsGraphSolid` API now retains six exact NURBS faces sharing
eight vertices and twelve edges. Canonical geometry and opposing coedges
certify this polynomial roof construction, with analytic volume and exact
bounds. A common dyadic display grid shares topological nodes across face
boundaries while retaining distinct normals at sharp joins. The native/WASM
browser demo edits dimensions and roof control offset. General rational
solid validation, arbitrary sewing, placement, Booleans and STEP interchange
remain unsupported. The estimate remains 15–25%, toward 80%.
See [polynomial graph solids](nurbs-graph-solid.md).

## Rigid placement of polynomial NURBS solids

The graph-solid wrapper now regenerates its retained B-rep under a composed
rigid placement, with unchanged exact volume and checked source/pcurve
agreement. Display preserves shared-node closure and transformed normals;
world-coordinate arithmetic allowances reject unresolved requests. Rotated
bounds are conservative control-hull enclosures rather than claimed exact
extrema. Native/WASM and the browser demo edit actual world placement. General
NURBS sewing, solid operations and interchange remain incomplete; the
estimate remains 15–25%, toward 80%.
See [graph placement](nurbs-graph-placement.md).

## Closed NURBS graph solid restriction

Exact source-UV restriction now constructs closed graph-solid subdomains
with a restricted roof, copied base and four ruled walls sharing curved
upper edges. Nested domains preserve original parameters and rigid placement;
analytic volume and conservative hull bounds are checked. Independent
triangle bounds cover roof and newly curved walls, with shared-node closure.
Native/WASM and browser editing exercise actual retained solid geometry.
This is a rectangular restriction, not general curved Booleans; broader
trims, sewing and interchange remain incomplete. The estimate remains
15–25%, toward 80%.
See [graph solid restriction](nurbs-graph-trim.md).

## Source-axis plane partition of NURBS graph solids

The scoped graph family now partitions along interior original U/V planes,
retaining two closed solids and an exact ruled section with a curved upper
edge. Both children preserve source parameters and placement; matching
cut-face controls, opposite orientations and conserved volume are checked.
Native/WASM and the browser inspect either actual result and reject boundary
contacts or unresolved cuts. Arbitrary planes, general curved Booleans,
sewing and interchange remain incomplete. The estimate remains 15–25%,
toward 80%.
See [graph plane partition](nurbs-graph-split.md).

## Closed NURBS graph solids with through openings

One strictly interior UV rectangle now creates a genuine through opening
with exact inner roof/base wires, curved rims and four inward cavity walls.
Canonical shared topology closes the genus-one shell. Stable positive-strip
volume accumulation handles thin remaining walls; a shared material-only
grid bounds roof and all walls while preserving rim closure. Native/WASM
and editable browser demonstrations cover retained rectangles and placement.
General tools, multiple openings, curved Booleans and NURBS interchange remain
incomplete. The estimate remains 15–25%, toward 80%.
See [graph through openings](nurbs-graph-hole.md).
## Scoped NURBS graph-solid point classification

Canonical graph solids, including restricted/placed solids and one retained
rectangular through opening, now expose Euclidean point classification.
Adaptive rational Bezier control-box lower bounds and actual surface witnesses
separate the boundary band before analytic material membership is used. Hole
cap interiors are excluded. Unsupported precision, threshold and resource
conditions return explicit errors rather than guesses; generic NURBS shell
classification remains unsupported. Native and browser queries share Rust.
See [scoped graph classification](nurbs-graph-classification.md).
The long-term estimate remains **15–25%**, toward the **80%** target.

## Scoped source-vertical material sections

Graph-solid wrappers now expose exact source-height material intervals and
retained cap crossing points, outward normals and face IDs. Source-UV
restriction, rigid placement and opening/exterior empty sections are supported.
Euclidean rectangle-perimeter contact checks reject ambiguous wall lines;
invalid metadata and insufficient precision return errors. The browser overlays
the actual query line, material segment and crossing normals, sharing Rust
with native JSON examples. Arbitrary directions and general NURBS intersection
remain unsupported. See [source-vertical sections](nurbs-graph-section.md).

## Scoped polynomial NURBS STEP export

Canonical graph-solid wrappers now write AP214 with actual spline degrees,
controls, original knots/multiplicities, shared edges and both face pcurves.
Inner cap wires and inward cavity walls preserve through openings. Independent
native basis/topology checks and an external reader's mm/metre face counts,
closed imported display, bounded approximate volume and hole crossings verify
the subset. Native/WASM and browser accepted-model downloads share one writer.
General rational shells, arbitrary trims and graph STEP import remain
unsupported. The estimate remains **15–25%**, toward **80%**.
See [graph-solid STEP export](nurbs-graph-step.md).

## Strict full-graph NURBS STEP import

Actual polynomial spline nets and all same-parameter face uses now reconstruct
one unplaced/full-domain six-face graph solid, retaining the parsed geometry
under exact canonical validation. Entity IDs, record/face ordering and cyclic
wire starts are not geometric identity. SI mm/metre conversion, no-snapping
rejection and bounded parser failure/recovery are independently verified.
Native/WASM and browser file/text import share the same Rust implementation.
The later holed-import milestone below adds one rectangular opening; placed,
restricted and general rational imports remain unsupported.
The estimate remains **15–25%**, toward **80%**.
See [strict graph STEP import](nurbs-graph-step-import.md).

## Strict holed-graph NURBS STEP import

A separate typed importer now retains the actual 16-vertex, 24-edge, 10-face
B-rep of a full, unplaced graph with one rectangular through opening. Annular
caps, inward walls, spline bases and both UV uses of each shared edge are
checked exactly. Bounded coefficient recognition accepts only a complete exact
geometry certificate; it never fits or snaps a nearby surface. Native/WASM
round trips and browser file/text loads cover signed, flat and tiny roof offsets,
unit conversion, reordered topology and malformed-input rejection. Placement,
restriction, multiple openings and general rational shells remain unsupported.
The estimate remains **15–25%**, toward **80%**.
See [holed graph STEP import](nurbs-graph-holed-step-import.md).

## Common checked graph STEP entry point

One bounded parse now selects plain or single-opening recognition from actual
shell topology and returns a typed `ImportedNurbsGraph` certificate. Shared
validation, volume, bounds, display, point classification, vertical section
and STEP export dispatch preserve the original exact geometry and rejection
policy. Native and safe WASM entry points accept both supported kinds without
caller-side STEP inspection. This improves integration, not geometric coverage:
placed/restricted imports and general shells remain unsupported.
The estimate remains **15–25%**, toward **80%**.
See [common graph import](nurbs-graph-step-auto-import.md).

## Scoped graph uniform-density mass properties

Certified graph solids and single-opening graph solids now expose volume and
world centroid from polynomial column moments. Retained UV rectangles and
four positive material strips support trimmed and off-centre holed parts without
mesh integration or near-equal volume subtraction. Rigid placement preserves
volume and transforms the centroid. Browser and native/WASM JSON show accepted
properties; generic rational-shell mass, mixed density, inertia and surface
area remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [graph mass properties](nurbs-graph-mass-properties.md).

## Scoped graph centroidal inertia

Polynomial graph solids and rectangular-through-opening solids now expose
centroidal inertia in world axes at uniform unit density (geometric mm⁵).
Direct central column moments and positive material strips preserve trims,
signed roof offsets and rigid placement without mesh or world-moment
subtraction. Browser diagonal inspection and complete native/WASM tensors
record explicit units/reference; unresolved numeric results remain errors.
General-shell mass, mixed density, principal-axis solvers and surface area
remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [graph inertia](nurbs-graph-inertia.md).
