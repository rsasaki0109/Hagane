# Development goal

## Actual horizontal torus child line/ray milestone

Actual accepted horizontal children now support mesh-free infinite lines and
forward rays with original nonunit parameters. The expanded full-source
local enclosure supplies outside endpoints to the checked actual-child segment
solver. Every mapped hit re-evaluates its actual restricted wall/cap face and UV,
measured distance and absolute normal-conditioned forward error. Ordered original
parameters, reconstructed points, physical interval widths/gaps and actual-child
membership are checked. At most four hits and two material intervals are returned.

Original full-body tolerance and anchor/world/frame guards precede cropping or
miss shortcuts. Ray origins inside the selected child clip their containing
interval to zero; other forward intervals remain. Lines can hit at parameter
zero; ray origins in the boundary band are refused. Zero/nonfinite directions,
remote or unrepresentable mappings, physical rims, contacts and unresolved
crossings fail explicitly. Source contacts remain conservative even behind a
ray or in the discarded halfspace.

The cached read-only session APIs, separate seven-value line/ray WASM buffers
and seventeen-value native examples use the actual selected component. Browser
child probes use private accepted selection and retain geometry, camera, STEP,
point/segment probes, source queries and the other line/ray result. Failed edits,
cuts and queries preserve accepted state; successful source replacement and child
selection changes clear stale browser results.

Independent tests verified circle-section roots, four hits/two intervals,
vertical wall/cap crossings, nonunit and reversed directions, hole/material/away
ray origins, positive interval clipping, actual face UVs and oriented normals,
arbitrary placement and three scales, cap/hole/extension/removed-side misses,
contact/backward-tangent/rim/boundary-origin/remote/nonfinite/zero refusal,
original relative tolerance and actual geometry/STEP/cache/query immutability.
Formatting, strict all-target Clippy, 37 torus native suites with 63 tests and
release WASM build passed. The full-source report remains byte-identical at
3,116,825 bytes, and every previous split geometry, mesh, volume, mass, bounds
and STEP field is unchanged except the scope description. All previous torus
WASM blocks and 216 new line/ray native-comparison cases passed on the first run.
Geometry/parameter/face/UV/normal fields retain strict 32-EPS comparisons; the
existing independently norm-checked, coordinate-propagated residual comparator
remains unchanged. Separate transport error recovery, accepted-cache retention
and successful source invalidation passed. The final entire expanded torus
browser runner passed, including selected-child line/ray original parameters,
actual wall/cap witnesses, inside-ray clipping, private selection, all prior
probes/metrics, rejected-state preservation, pixel/camera guards, private STEP
export and mobile layout. The linked screenshot was captured from the actual
running lower-child line/ray demo and visually inspected.

Repeated child cuts, arbitrary oblique torus cuts, STEP import and general torus
Booleans remain future work. Overall completion remains an engineering estimate
of about **20% (15–25%)**, toward the **80%** goal.


## Actual horizontal torus child finite-segment milestone

Actual cached horizontal children now support mesh-free finite segment
intersections. Certified source torus quartic material intervals are clipped
against the actual child's finite annular cap and retained halfspace; all
returned hits carry actual restricted wall or cap face IDs, original UVs,
evaluated points, measured distances and oriented normals. Parameters stay in
the original segment's [0,1] range, with at most four hits and two material
intervals; inside starts/ends and reversal are supported.

Original full-body tolerance and world/frame guards precede shortcuts. Absolute
forward-error, physical root separation and interval membership checks reject
shallow crossings, physical trim rims, boundary endpoints and unresolved contact.
Source solver refusals remain conservative for discarded tangencies or source
boundary endpoints. Annular holes and plane extensions supply no ghost cap hit.

The read-only session API, independent seven-value WASM buffer and seventeen-value
native example use the accepted actual split pair. Browser child-segment probes
use private accepted selection and preserve geometry, camera, source queries,
child point results and STEP. Successful source replacement invalidates the pair;
failed edits, cuts and queries preserve it.

Independent tests verified circle roots, horizontal four-hit/two-interval paths,
vertical cap/wall crossings, reversed and inside-clipped segments, actual face
UVs/normals, arbitrary placement and three scales, holes/plane extensions and
removed-side misses, original relative tolerance, contact/tiny/remote/nonfinite
refusal, and actual geometry/STEP/query/cache immutability. Formatting, strict
all-target Clippy, 33 torus native suites with 57 tests and release WASM build
passed. The 3,116,825-byte full-source JSON remains byte-identical; every previous
split geometry, display, bounds, volume, mass and STEP field is unchanged except
the scope description. All prior torus WASM blocks and the new cached-child
segment blocks passed. Geometry/parameter/UV/normal report parity retains the
original strict 32-EPS comparison; measured residual distance is independently
verified against each platform's actual serialized points, then compared with
coordinate error propagated through the norm. No kernel distance rounding or
existing helper/assertion relaxation was introduced. The final entire expanded
torus browser runner passed, including the reported residual-rounding case,
actual child wall/cap witnesses, private selection, rejected edit/query/cut
preservation, source/child metrics and probes, camera/pixel guards, private STEP
export and mobile layout. The linked screenshot was captured from the actual
running lower-child cap/wall segment demo and visually inspected.

Repeated child cuts, arbitrary oblique torus cuts and general torus Booleans
remain unsupported; child line/ray queries are implemented above. Overall completion remains an
engineering estimate of about **20% (15–25%)**, toward the **80%** goal.


## Actual horizontal torus child mass milestone

Actual validated horizontal children now expose uniform-density world centroids
and full world-axis centroidal inertia tensors. Analytic disk-segment integration
uses stable bounded binomial series for thin caps, checked positive products and
explicit conditioning/representability refusal. No mesh integration or source
solid substitution is used. Optional split reports carry metric errors without
discarding valid geometry; the browser presents selected-child metrics separately
from full-source metrics, using its private accepted pair.

Independent tests checked half closed forms, independent moment quadrature,
positive/negative and thin cuts, arbitrary rigid placement, three scales,
weighted centroid and full parallel-axis inertia conservation, geometry/STEP
immutability, unresolved policies and admitted geometry with inertia overflow.
Formatting, strict all-target Clippy, 29 torus suites with 51 native tests and
release WASM build passed. The 3,116,825-byte full-source report is byte-identical;
all previous split geometry, mesh, bounds, volume and STEP fields are unchanged.
All torus WASM blocks passed with unchanged strict native parity, including
actual child centroid and world-tensor conservation and previous query/cache
error-recovery checks. The entire expanded torus browser runner passed, including
independent child metric conservation, private accepted selection, separate source
and child panels, reset and rejected-edit preservation, camera/pixel guards and
private STEP export. The linked screenshot was recaptured from the actual running
WASM child metric and cap-query demo and visually inspected.

Repeated child cuts, arbitrary oblique torus cuts and general torus Booleans
remain unsupported; child finite segment/line/ray queries are implemented above. Overall completion remains
an engineering estimate of about **20% (15–25%)**, toward the **80%** goal.

[Horizontal torus child point queries](docs/nurbs-torus-horizontal-split.md#mesh-free-child-point-queries)
now use the accepted actual split pair with finite annular caps and retained
NURBS walls. Mesh-free witnesses carry actual child face IDs, original UVs,
evaluated points, measured distances and oriented normals.

## Actual horizontal torus child-point milestone

Finite Euclidean meridian-arc and annular-cap distances prevent false boundary
hits on cap holes, plane extensions or discarded walls. The original full-body
policy and world/frame precision guards precede shortcuts. Physical cut-rim
boundary ambiguity and unresolved thresholds return explicit errors.

The session builds one actual split pair and commits it only after both reports
serialize successfully. Failed cuts, display requests, model edits and queries
preserve the accepted pair and full source; a successful source update invalidates
the pair. Separate four-value WASM transport and a fourteen-value native example
query actual cached children. Browser selection chooses the private accepted
child while full-source point/segment/line/ray probes retain their meaning.

Formatting, strict all-target Clippy, 26 torus native suites with 45 tests and
release WASM build passed. Independent tests checked both children at zero and
nonzero heights, arbitrary placements and three scales, actual face/UV/normals,
physical normal offsets, annular holes and finite-plane extensions, original
relative tolerance, explicit rim refusal and source/body/STEP immutability.
The entire previous torus WASM blocks plus child-point transport and strict
native parity passed. The full-source report remains byte-identical at 3,116,825
bytes; all previous split geometry, mesh, metrics and STEP values are unchanged
except the scope description. The entire expanded torus browser runner passed,
including private child selection, actual cap/wall witnesses, native parity,
failed edit/query preservation, camera/pixel guards and private STEP export.
The README-linked screenshot was captured from the actual running child probe.
Repeated child cuts and arbitrary oblique torus cuts remain unsupported;
child mass moments and segment/line/ray queries are implemented above.


[Horizontal torus splitting](docs/nurbs-torus-horizontal-split.md) produces two
actual closed genus-one B-rep solids by restricting the source NURBS patches and
adding annular plane caps. Each child supports checked volume, conservative
bounds, conforming display and export of its actual STEP geometry/topology.

## Actual horizontal torus plane-split milestone

A local height cuts perpendicular to the source axis, including arbitrary rigid
source placement. Literal zero retains whole minor quarters; resolved nonzero
cuts retain connected restricted strips in their original UV domains. There is
no near-zero snapping. Every cut clearance, annulus width and partial strip
must resolve under the original full-body policy and world arithmetic.

Exact source parameter curves supply the shared cut rims and meridians. Caps
retain matching rational weights/knots and affine-projected original rim control
points. Complete actual-body coefficients/topology are certified against the
source restrictions; every shared edge has two opposing oriented uses. Positive
child volumes use guarded disk-segment integration and a cancellation-resistant
small-cap series. Display evaluates actual restricted surfaces and shares exact
edge samples with the annular cap, using a whole-triangle angular Hessian bound
plus frame/restriction/evaluation/cache arithmetic reserves.

The read-only session serializes both child meshes, actual B-reps and STEP
before success. Its separate two-value WASM transport rejects malformed input
without replacing the accepted full source. The browser can view either child,
export its private accepted STEP and restore the source view. Source metrics,
STEP and all existing probes are explicitly identified as full-source operations;
preview children do not claim unimplemented child query behavior. Failed cuts,
display requests or model edits preserve the last accepted view and exports.

Formatting, strict all-target Clippy, twenty-two related native suites with
38 tests and the release WASM build passed. Independent smooth-coordinate
numerical integration checked both child volumes, including admitted thin caps,
arbitrary poses and three scales. Actual restricted UVs, implicit torus geometry,
whole cap boundaries, oriented shared topology, actual STEP and source immutability
passed. Display tests checked dense whole-triangle error, real normals, annular
void intrusion, exact welded two-use edge incidence and genus one. Session tests
verified unchanged actual source B-rep/STEP and all previous query strings after
successful previews and rejected geometry/display requests. The previous full
model report remained byte-identical: 3,116,825 bytes, with no scope exclusion.
The entire previous torus query blocks and new horizontal split WASM tests passed
with strict native report/STEP parity, cut/pose/scale coverage, source query
retention, independent transport recovery and fresh-instance refusal before
source acceptance.
The entire expanded torus browser script passed against the final artifact,
including actual closed-child geometry, independent volume/chord evidence,
strict native parity, private child/source exports, explicit full-source query
labels, cut/model failure preservation and responsive orbit. Its production
screenshot shows the actual lower child and annular cap and was inspected.
Temporary dependency linkage and focused harnesses were removed.

General oblique torus cuts, repeated child cuts, child mass/query operations,
generic torus STEP import and general Booleans remain unsupported. No new
dependency or external source was introduced. Broad completion remains 15–25%
toward the standing 80% goal.

[Torus lines and rays](docs/nurbs-torus-line.md) now preserve original nonunit
parameters and return every resolved actual face/UV crossing and material
interval. Forward rays clip inside-origin intervals to zero and retain a second
interval across the central hole. Accepted-body native/WASM/browser queries
never use the display mesh as a geometric source.

## Full regression checkpoint: torus line and ray queries

On frozen `87645d0`, the complete native run passed 345 suites with 1,255
passed tests, zero failures and zero ignored tests, including two documentation
tests. The entire WASM regression script also exited successfully, including
all prior modeling/interchange workflows and actual accepted torus point,
segment, line and ray probes. Horizontal plane splitting developed in a separate
checkout throughout; it is not part of this checkpoint. The complete expanded
torus line/ray browser runner had already passed separately.

## Original-parameter torus line/ray milestone

Original anchor, full-source tolerance, represented frame and world arithmetic
checks run before enclosure miss shortcuts. Expanded local ring bounds crop a
finite supporting segment; both crop endpoints must classify Outside against
the actual torus. The existing transverse quartic solver supplies all roots.
Mapping back to the caller's nonunit direction checks physical reconstruction,
actual surface residuals, interval order and parameter representation.

Rays classify the original origin, reject its boundary band and check interval
straddling and physical parameter signs before forward clipping. Infinite lines
can use a transverse boundary anchor. The complete supporting line must resolve:
backward tangency or unresolved roots also refuse a ray. Zero/nonfinite or
unrepresentable directions, remote anchors, contact, unresolved crop/mapping
and world/frame precision return explicit errors.

Separate line and ray session methods and six-value WASM transports read the
actual accepted body and policy. Browser queries retain point/segment results,
mesh, camera and STEP after failures or unaccepted edits. Successful model edits
clear stale probes. Partial query buffers remain independent across query kinds.

Formatting, strict all-target Clippy, seventeen related native suites with
30 tests and the release WASM build passed. Independent tests checked circle
section roots, original positive/negative nonunit directions, inside/hole/behind
ray origins, all material intervals, arbitrary poses and three scales, actual
UV/normal witnesses and unchanged actual B-rep/STEP on rejection. The prior model
report retained every geometry, mesh, metric, bounds and STEP value excluding
updated scope metadata. The entire previous torus/point/segment and new line/ray
WASM blocks passed with strict native parity, the posed cardinal-seam fixture,
cross-buffer recovery and fresh-instance refusal before model acceptance.
The entire expanded browser script passed with actual witnesses, strict pixel,
camera and private accepted-export preservation, dirty model queries and mobile
orbit. Its real production screenshot with line/ray results was inspected.
Temporary dependency linkage and focused harnesses were removed.

General closest points, trimming, torus STEP import and torus Booleans remain
unsupported. No new dependency or third-party source was introduced. Broad
completion remains 15–25% toward the standing 80% goal.

[Finite torus segment queries](docs/nurbs-torus-segment.md) return up to four
ordered actual face/UV hits and two material intervals from the accepted Rust
B-rep, including inside-endpoint clipping and reversed segments. Meshes do not
supply intersections.

## Full regression checkpoint: finite torus segments

On frozen `fc02909`, the complete native run passed 341 suites with 1,248
passed tests, zero failures and zero ignored tests, including two documentation
tests. The entire WASM regression script also exited successfully, including
all prior modeling/interchange workflows and actual accepted torus point and
finite segment probes. Line/ray development remained in a separate checkout;
this checkpoint does not include those new APIs. The complete expanded torus
browser runner for the segment milestone had already passed separately.

## Finite torus segment milestone

The independently derived scaled quartic uses guarded Bernstein derivative
subdivision and one-Lipschitz boundary-clearance collars. Each accepted crossing
has an opposite-sign physical root bracket at most the original absolute linear
tolerance divided by sixteen, plus an actual NURBS witness and entering/exiting
normal. Full-source contact bands, world/frame precision checks, shallow-angle
forward guards and bounded work reject unresolved inputs explicitly. Tangencies,
boundary-band endpoints, tiny/nonfinite segments and close roots refuse rather
than returning an incomplete result.

Native/session/example, a separate six-value WASM transport and the browser
finite segment probe read the actual accepted typed solid and policy. Failed
queries and unaccepted edits preserve the geometry, mesh, point probe and STEP.
Successful model replacement clears stale point and segment results.

The initial strict native/WASM comparison exposed different adjacent face IDs
at a posed cardinal seam despite identical root parameters and physical witness
agreement. Arithmetic-scale seam canonicalization now chooses the same actual
quarter start and retains the measured surface-distance check. The exact failing
fixture remains in the strict native/WASM and browser regression tests; parity
assertions were not weakened.

Formatting, strict all-target Clippy, thirteen related native suites with
23 tests and the release WASM build passed. Independent tests checked analytic
circle-section roots, hole misses, inside-endpoint clipping, reversal, poses and
three scales, actual face/UV/normal witnesses, full relative bands, precision
refusal and unchanged actual B-rep/STEP. The final model report retained every
prior geometry, mesh, metric, bounds and STEP value, excluding updated scope
metadata. The entire previous torus/point and new segment WASM blocks passed,
including strict posed-seam native parity and fresh-instance refusal before
model acceptance. The first parity failure and its diagnosis remain preserved.
The entire expanded torus browser runner passed against the final artifact,
including the exact seam reproduction, actual face/UV witnesses, three scales,
strict camera/pixel/private export preservation and responsive orbit. Its actual
production screenshot shows the accepted posed torus, four hits and two material
intervals and was inspected. Temporary dependency linkage was removed.

General closest points, trimming, torus STEP import and torus
Booleans remain unsupported. No new dependency or external source was introduced.
Broad completion remains 15–25% toward the standing 80% goal.

[Mesh-free torus point queries](docs/nurbs-torus-classification.md) distinguish
the tube material, central hole and boundary with actual retained NURBS face/UV
witnesses. Native and browser queries read the accepted Rust solid; rejected
edits and query failures preserve geometry, mesh and STEP.

## Full regression checkpoint: accepted torus point queries

On frozen `e306821`, the complete native run passed 336 suites with 1,240
passed tests, zero failures and zero ignored tests, including two documentation
tests. The entire unchanged WASM regression script also exited successfully,
including all prior workflows and accepted torus point probes. Finite segment
development remained in a separate checkout throughout these runs.
The entire unchanged all-pages browser rerun also exited successfully after
the existing build-script sample assets were restored. The tested WASM artifact
remained unchanged. The initial missing-fixture failure remains recorded below.

## Accepted torus point-query milestone

The full-body tolerance policy bounds ideal Euclidean tube distance with query
world-coordinate and represented-frame arithmetic reserves before shortcuts.
Boundary success requires an actual rational surface point, normal and measured
distance within the band. Nonfinite, remote and unresolved threshold inputs
refuse explicitly. The torus axis and tube center circle classify without
division by zero. No display mesh provides the classification.

The new accepted-body session serializes the complete geometry/display/STEP
report before committing an edit. Read-only point queries retain that actual
typed body and its accepted policy. The separate three-value WASM transport
rejects malformed buffers without changing the model or the eight-value edit
buffer. Browser dirty fields do not replace the accepted query source.

Formatting, strict all-target Clippy, eight related native suites with fifteen
tests and the release WASM build passed. Independent tests checked all sixteen
patches, seams, actual UVs/normals, true normal offsets across poses and three
scales, full-source relative bands, precision refusal and immutable actual
B-rep/STEP. The prior model report retained every geometry, mesh, metric, bounds
and STEP value, excluding the updated scope description. The complete old torus
and new query WASM blocks passed, including native query parity and a separate
fresh-instance rejection before model acceptance. The entire expanded torus
browser script passed with strict geometry/camera/pixel/export preservation,
and its actual Boundary-witness screenshot was inspected.

An additional unchanged all-pages browser run reached its STEP sample page
and failed because this isolated checkout lacked generated sample assets.
Existing build-script fixture copies restored them without changing the tested
WASM. The entire unchanged legacy rerun subsequently passed, including its final
metre-unit STEP page. No assertion or tested artifact was changed.

General closest points, trimming, STEP
import and Booleans remain unsupported. No new dependency or external code was
introduced. Broad completion remains 15–25% toward the standing 80% goal.

## Full regression checkpoint: rational ring torus

The frozen `1e33162` implementation passed the complete native suite:
332 suites, 1,233 tests, zero failures and zero ignored tests, including two
documentation tests. The entire `scripts/test-wasm.mjs` runtime script also
completed with exit zero, including every existing domain and the new torus
geometry, mesh, mass, STEP and transport checks. Next-milestone development
remained in an isolated worktree throughout these regressions.

This verifies the documented regular-ring domain; torus STEP import, queries
and Booleans remain unsupported in this checkpoint. The broad completion
estimate remains 15–25% toward the standing 80% target.

[Regular ring tori](docs/nurbs-torus.md) extend exact NURBS B-rep topology to
surfaces curved in both parameter directions. The actual shared solid supplies
analytic volume and centroidal inertia, bounded conforming display and exact
STEP export in native Rust and the interactive WASM/browser demo. Horn/spindle
tori, torus STEP import, line queries, trimming and Booleans remain unsupported.

## Rational ring torus milestone

Sixteen degree-(2,2) rational tensor patches retain sixteen vertices,
thirty-two shared quadratic edges, oriented affine surface boundaries and a
closed genus-one shell. The typed certificate rejects modified geometry and
unresolved tube/ring clearance or world precision. Display evaluates actual
surfaces and shared edges, uses an interior-triangle error bound with arithmetic
reserves and refuses requests beyond its 32,768-triangle budget.

Formatting, strict all-target Clippy, four related native suites with eight
tests, and the release WASM build passed. Independent tests checked actual
coefficients, dense surface/triangle interiors, bit-identical seam closure,
posed/scaled volume, all world inertia entries and STEP. Optional inertia
overflow keeps valid geometry, volume, mesh and STEP with an explicit metric
error. The combined WASM script passed the entire new torus block and prior
interior-plane/retained-bore/adapter blocks. The entire new browser script passed
geometry, metrics, native STEP parity, strict rejected-edit preservation,
private accepted export, orbit/zoom and mobile layout. The actual production
screenshot was inspected after restoring the normal backdrop.

Initial browser launch failed because its bundled browser cache was absent;
the runner now uses the existing system-browser fallback. Preliminary native
parity runs also detected an older WASM artifact missing new report metadata;
after a fresh build, final combined WASM and entire browser runs exited zero.
Assertions were retained. No OCCT code or new dependency was introduced.
The broad completion estimate remains 15–25% toward the standing 80% target.

## Full regression checkpoint: certified retained-bore adapters

The frozen `b44614f` implementation passed the complete native suite:
328 suites, 1,225 tests, zero failures and zero ignored tests, including two
documentation tests. The entire `scripts/test-wasm.mjs` runtime script also
completed with exit zero, covering the existing primitives, NURBS, STEP,
workflows and the new canonical/oblique/slab bore adapters. Development of the
next milestone used an isolated worktree while these checks ran.

This verifies the implemented bounded domains; general curved Booleans and
general bored-frustum STEP import remain unsupported. Overall completion
remains an engineering estimate of 15–25% toward the standing 80% target.

[Editable through-bore histories](docs/nurbs-frustum-workflow.md) retain the actual
bored solid through hole-radius edits, component selection, incremental rebuilding,
Undo/Redo and exact STEP export. Mesh-free point classification measures distance
to finite outer/inner walls and annular caps. A read-only browser probe uses
the accepted selected component without changing geometry or history. Exact
finite segment queries return both material intervals across a hole, with actual
face/UV witnesses. [Oblique segment queries](docs/nurbs-frustum-oblique-segment.md)
now clip against the actual cut cap and map restricted-wall UVs.
[Line/ray queries](docs/nurbs-frustum-retained-line.md) preserve original nonunit
parameters and all retained material intervals. [Axial bore partitions](docs/nurbs-frustum-bore-partition.md)
retain the hole in every child and permit further selected-child partitions.
[Oblique bore cuts](docs/nurbs-frustum-bored-oblique.md) now retain annular
children through a complete interior plane section.
[Checked analytic mass](docs/nurbs-frustum-bored-oblique-mass.md) gives actual
child centroids and inertia with explicit conditioning errors.
[Finite point queries](docs/nurbs-frustum-bored-oblique-classify.md) retain the
hole and certify actual annular cut-cap witnesses.
[Finite bored-oblique segments](docs/nurbs-frustum-bored-oblique-segment.md)
retain both material intervals and actual restricted-face UVs. Line/ray queries
also preserve original parameters on these children.
[Repeated parallel cuts](docs/nurbs-frustum-parallel-slabs.md) retain actual
slab boundaries and holes. [Strictly interior skew cuts](docs/nurbs-frustum-interior-plane-splits.md)
now retain differently oriented, uniformly ordered plane boundaries and holes
in separate plane-bounded types. [Retained coaxial drilling and enlargement](docs/nurbs-frustum-retained-bore.md)
now add holes or strictly enlarge them on these actual plane-bounded bodies,
with optional closed removed partitions. Checked adapters also drill after
single oblique and parallel cuts and enlarge canonical or retained holes.
Intersecting boundary cuts, off-axis/blind frustum bores and general
bored-frustum STEP import remain unsupported.
[Exact cardinal bored STEP import](docs/nurbs-frustum-bored-step-import.md)
retains actual annular topology and coefficients in its separate bounded domain.
Overall completion remains an engineering estimate of **15–25%**, toward the **80%** goal.

[Coaxial frustum through bores](docs/nurbs-frustum-bore.md) remove an actual
cylindrical tool and retain a closed rational genus-one B-rep, inward cavity
walls and annular cap wires. Positive analytic volume/moments, bounded display
and exact STEP export work without mesh CSG. Off-axis and blind frustum bores,
general bored-frustum import and general curved Booleans remain unsupported.

[Editable oblique frustum workflows](docs/nurbs-frustum-workflow.md) connect
world-plane cuts to actual typed child selection, incremental rebuilding,
Undo/Redo and exact per-child STEP export. Optional mass failures retain geometry;
skew further cuts of oblique children remain explicit errors. Rust components now
use canonical, oblique, bored and bored-oblique variants; existing JSON documents remain supported.

[Mesh-free oblique-child point queries](docs/nurbs-frustum-oblique-classify.md)
classify the retained finite solid with checked source and cut-cap witnesses.
A separate browser probe uses the accepted model; unresolved elliptical-rim
distances fail explicitly while geometry and export remain available.

[Oblique frustum splits](docs/nurbs-frustum-plane-split.md) produce two actual
closed rational B-rep children with new planar cut caps, analytic volumes,
conforming bounded display and per-child STEP export. The initial domain keeps
the complete cut strictly between the original caps.
[Analytic child mass properties](docs/nurbs-frustum-oblique-mass.md) provide
checked world centroids and centroidal inertia; unresolved metrics fail explicitly
while geometry remains available. Skew further cuts remain unsupported. This extends actual curved 3D
partition infrastructure without claiming general NURBS Booleans.

[Closed oblique frustum sections](docs/nurbs-frustum-plane-section.md) construct
actual quadratic rational edges and cubic same-parameter source-face pcurves.
The closed section has shared vertices and bounded display; it is not a solid
cut. Cap crossings, contact and unresolved precision are explicitly unsupported.
This advances curved intersection prerequisites; general 3D Booleans remain future work.

[Frustum line and ray queries](docs/nurbs-frustum-line.md) retain original
nonunit parameters and actual rational face UV witnesses. A checked finite-body
enclosure feeds the segment solver; inside rays and resolved misses are explicit.
The native/WASM/browser demo preserves the actual B-rep and STEP on rejection.
This is intersection infrastructure; general curved Booleans remain unsupported.

[Finite frustum segment queries](docs/nurbs-frustum-segment.md) return mesh-free boundary hits with
actual face UV witnesses and a checked material interval. Native/WASM/browser
demos retain the original B-rep; contacts and insufficient precision are explicit
errors. This supplies intersection infrastructure without claiming a Boolean cut.
Rejections distinguish boundary endpoints, tangency, overlaps and coordinate
precision, with input-change guidance propagated through native/WASM/browser.

[Editable rational frustum workflows](docs/nurbs-frustum-workflow.md) connect actual cardinal-frame
stock or STEP input to axial partitions and explicit component selection.
Native and WASM sessions reuse unchanged B-reps, validate outputs before commit
and support bounded Undo/Redo. General curved Booleans remain unsupported.
The dedicated browser editor supports JSON documents, physical preset controls,
explicit selection, Undo/Redo and per-component STEP download.
A separate axis-angle stock node supports checked arbitrary rigid orientation;
STEP input retains its documented cardinal-frame restriction.

[Exact rational NURBS frustum lofts](docs/nurbs-frustum.md) add closed ruled
surfaces beyond height graphs, with same-parameter rational boundaries, validated
mass/inertia, bounded display and exact STEP export. This typed positive-radius
coaxial domain does not claim arbitrary lofts, apex shapes or general NURBS Booleans.
Mesh-free world-point classification now checks Euclidean distances to finite
side/cap/rim boundaries; unresolved precision is explicitly refused.
[Axial frustum partitions](docs/nurbs-frustum-split.md) return two validated closed children,
actual rational section curves and per-child STEP. A separate
[oblique split API](docs/nurbs-frustum-plane-split.md) now admits closed cuts
strictly between the source caps; general oblique partitions remain unsupported.
The batch axial API accepts 1–16 ordered cuts, certifies every part against
the original source and refuses unresolved intervals atomically. Its native
example exports all parts. The [batch demo](docs/nurbs-frustum-partitions.md)
connects actual parts to native/WASM reports and browser selection/STEP export.
The dedicated [frustum STEP reader](docs/nurbs-frustum-step-import.md) now
recognizes strict unplaced mm bodies, retains actual rational geometry and UV
boundaries, and checks exact serialized LINE/affine preimages before admission.
The separate [translated importer](docs/nurbs-frustum-translated-step-import.md)
accepts identity-axis translations and axial upper children with exact retained
geometry. The separate [cardinal importer](docs/nurbs-frustum-cardinal-step-import.md)
accepts 24 exact signed X/Y/Z bases. Arbitrary-angle placements and general
rational STEP remain unsupported.
The cardinal demo now connects actual X/Y/Z-oriented STEP to full-frame
native/WASM reports and browser import, display and exact re-export.
The separate translated demo now shares native/WASM reports and browser
file/text import, actual B-rep display and exact re-export.

[Editable multi-component prism workflows](docs/prism-workflow.md) connect
exact stock and actual STEP import to difference/intersection/union nodes,
explicit component selection, incremental prefix reuse, Undo/Redo and per-part
STEP export. Empty and disconnected results remain explicit; general compound
and 3D operations remain unsupported.

[Holed-tool prism Booleans](docs/prism-region-booleans.md) now classify
material regions in both operands, returning exact difference, intersection and
union components including retained central islands. The finite line/quarter-arc,
same-axis/same-cap-interval domain keeps contact overlays and general 3D Boolean
operations unsupported. Earlier no-tool-holes APIs retain their restrictions.

[Exact line/arc prism Booleans](docs/prism-arc-line-booleans.md) now return
all difference, intersection and union components when circular-arc tools cross
line/arc stock boundaries. Analytic circle/circle roots extend the actual B-rep
arrangement; the same-axis, same-cap-interval domain rejects contact overlays
and general curved 3D Booleans.

The latest [exact curved prismatic partition](docs/prism-convex-partition.md)
constructs difference/common B-reps when convex polygon tools cross cylindrical
stock walls. Analytic line/circle arrangements return all actual components and
preserve restricted curves/pcurves; empty and disconnected results are explicit.
The current domain shares one physical axis and cap interval and rejects contacts,
tangency/vertex overlays and general curved 3D Booleans. This advances the reusable
intersection-graph and sewing prerequisite beyond contained circular bores.
[Editable curved-stock blind histories](docs/workflow-normal-blind.md) remain
available with incremental replay, Undo/Redo and STEP export.

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

Still incomplete: general NURBS solid integration, arbitrary trimmed NURBS
tessellation, general Booleans, fillets/chamfers, healing, and broader STEP
interchange. Legacy uniform NURBS grids have no certified chord-error bound;
rectangular retained faces have bounded display with an engineering floating-point
allowance rather than formal interval certification. See
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
See [rectangular NURBS face](docs/nurbs-face.md).


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
[bounded surface display](docs/nurbs-surface-tessellation.md).


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
toward the **80%** target. See [surface extraction](docs/nurbs-surface-extraction.md).


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
**15–25%**, toward the **80%** target. See [C0 face display](docs/nurbs-surface-crease.md).

## Exact rectangular NURBS restriction

`NurbsSurface::restricted` retains an original-parameter UV rectangle using
homogeneous knot insertion and tensor control selection, without fitting or
mesh clipping. `NurbsFace::trimmed` validates the source and regenerates exact
rational boundary edges, shared corner references and affine pcurves on the new
clamped surface, retaining face orientation. Interior C0 creases keep their
one-sided shading normals; cuts at a crease use the retained inward derivative.

Native tests compare source/restricted geometry and partials, boundary identity,
orientation, nonunit domains and invalid/resource cases. The native/WASM/browser
demo edits bounds and precision and preserves accepted geometry after rejected
inputs. Bounded display concerns the retained rational surface, with existing
engineering arithmetic guards rather than formal interval certification.

This supplies rectangular restrictions, not arbitrary trim curves or holes.
Cross-face sewing, global regularity, closed NURBS solids, intersections,
general curved Booleans and NURBS STEP remain incomplete. The estimate remains
**15–25%**, toward **80%**. See [rectangular restriction](docs/nurbs-surface-trim.md).

## Rectangular inner wires on rational faces

`NurbsHoledFace` retains a rational surface, rectangular outer wire and separated
rectangular UV inner wires. Exact isocurves, affine pcurves, shared corners and
clockwise hole traversal define the B-rep independently of display. Hole
coordinates refine the rational source into a conforming tensor grid; display
retains whole cells outside the openings with original UVs, split C0 normals
and existing per-cell geometric bounds. It does not use mesh Booleans as CAD
geometry. UV separation has its own parameter arithmetic guard, independent
of physical-length tolerance. Contact, overlap and resource failures are explicit.

Native topology/coverage/bound tests and the native/WASM/browser fixture verify
the retained holes, bounded triangles, editing and failure recovery. This remains
an open face without global regularity certification. Curved/nonrectangular
trims, cross-face sewing, closed NURBS solids, intersections, general curved
Booleans and NURBS STEP remain future work. The long-term estimate remains
**15–25%**, toward **80%**. See [UV openings](docs/nurbs-surface-hole.md).

## Evaluate retained rational face cells before removed regions

Holed-face tessellation now removes exact excluded span cells before bounds,
subdivision and normal evaluation. This eliminates false display failures from
sampled singularities inside removed holes and counts only retained cells toward
the display budget. Exact surfaces, inner wires, original UV values, split C0
normals and per-cell Bernstein bounds remain. Whole-source refinement/extraction
limits and engineering coordinate/weight guards still apply.

Native tests cover excluded versus retained singularities, resource limits and
topology. The native/WASM/browser polynomial fixture retains an exact opening
around a singular source point and verifies analytic geometry, normals, bounds,
editing and recovery. This is display of the retained face, not shape repair or
a global regularity certificate. Arbitrary trims, cross-face sewing, closed
rational solids and general curved Booleans remain incomplete. The estimate
remains **15–25%**, toward **80%**. See [retained-domain display](docs/nurbs-surface-material.md).

## Exact straight UV edges on rational surfaces

`NurbsSurface::parameter_curve` now composes an affine UV segment with the
homogeneous tensor surface, splitting at original knot crossings and retaining
an exact mathematical piecewise rational curve over T=0..1. Constant axes are
contracted, reversed paths preserve their parameter direction, and combined
degrees or unresolved knot crossings beyond the supported domain fail explicitly.
`NurbsSurfaceEdge` retains the supporting surface, real B-rep edge, two vertex
references and same-parameter affine pcurve, independently of display sampling.

Native tests cover independent rational formulas, one-sided source chain rules,
nonunit domains, reversed paths, topology and bounded polylines. Native/WASM and
browser fixtures show diagonal edges across smooth and C0 knot boundaries, with
editable endpoints and rejected-input recovery. Composition/refinement uses
checked `f64` arithmetic, not fitting or formal interval certification.

This provides a prerequisite for straight-boundary surface trims; it does not
construct arbitrary trimmed faces, sewing, closed rational solids, intersections,
general curved Booleans or NURBS STEP. The estimate remains **15–25%**, toward
**80%**. See [surface edges](docs/nurbs-surface-edge.md).

## Closed convex UV surface wires

`NurbsSurfaceWire` now retains exact lifted edges, affine pcurves and shared
closing vertex indices for 3–64 strictly convex UV corners, in either traversal
direction. Validation rejects unsupported polygons and corrupted topology.
Native tests and an executable example verify rational edge/surface identity
and bounded boundary display. Face interior tessellation, arbitrary trim loops,
sewing and closed NURBS solids remain incomplete. The long-term estimate remains
15–25%, toward 80%. See [surface wires](docs/nurbs-surface-wire.md).

## Convex UV polygon faces

`NurbsPolygonFace` retains an open face with an exact convex straight-UV outer
wire, shared boundary topology and an independent signed face orientation.
A scoped affine-patch interior tessellator and OBJ example verify analytic area
and normals. Curved face construction is supported, while curved interior display
returns an explicit unsupported error. General trims, holes, sewing and closed
rational solids remain incomplete. The estimate remains 15–25%, toward 80%.
See [polygon faces](docs/nurbs-polygon-face.md).

## Bounded curved bilinear polygon display

Convex UV faces now have conforming bounded interior display for one equal-weight
bilinear patch, including genuinely curved saddles. Shared midpoint indices,
original UVs and per-triangle engineering bounds preserve the retained region.
Native independent formula/coverage/orientation tests and an actual OBJ demo
verify this scope. General rational/higher-degree/multi-span polygon display,
holes, sewing and closed rational solids remain incomplete. Native/WASM builds
share the kernel; no dedicated browser export was added. The estimate remains
15–25%, toward 80%. See [bounded polygon display](docs/nurbs-polygon-bounded.md).

## Positive-weight rational bilinear polygon display

Bounded polygon tessellation now accepts nonuniform positive weights on one
bilinear patch. Homogeneous derivative bounds and a barycentric Taylor remainder
replace the polynomial-only twist bound, with explicit numerical-conditioning
and budget rejection. Independent rational formulas and common weight scaling
tests verify the actual implementation; the OBJ demo now uses nonuniform weights.
Higher-degree/multi-span display, holes, sewing and closed rational solids remain
incomplete. The estimate remains 15–25%, toward 80%. See [bounded polygon
display](docs/nurbs-polygon-bounded.md).

## High-degree rational polygon display

`NurbsPolygonFace::tessellate_bounded` extends conforming polygon display to one
positive-weight high-degree Bezier patch. General Bernstein first/second derivative
control nets supply checked Hessian bounds. Independent biquadratic and asymmetric
cubic/linear rational formulas test every emitted triangle bound; a weighted
biquadratic OBJ demo exercises actual retained-face display. The bilinear API
retains its original contract. Multiple spans, general trim loops, holes, sewing
and closed rational solids remain incomplete. The estimate remains 15–25%,
toward 80%. See [Bezier polygon display](docs/nurbs-polygon-bezier.md).

## C1 multi-span rational polygon display

Bounded polygon display now covers structurally C1 source knots through scaled
per-Bezier-patch derivative bounds. Independent Cox–de Boor evaluation verifies
triangles crossing both U and V knots in nonunit domains; the OBJ demo exports
an actual four-span rational face. Structural C0 knots are explicitly rejected,
including geometrically smooth over-multiplied knots. Full-source extraction,
derivative work and display budgets remain checked. C0 polygon interiors, holes,
general trims, sewing and closed rational solids remain incomplete. The estimate
remains 15–25%, toward 80%. See [C1 polygon display](docs/nurbs-polygon-multispan.md).

## Crease-aware rational polygon display

`tessellate_crease_bounded` splits convex UV triangle domains at structural C0
knots, sharing intersection/midpoint identities and geometric positions while
retaining explicit one-sided normals. Independent piecewise rational bounds,
coverage/connectivity and analytic normals verify crossed creases and boundary
ownership. An OBJ demo exports actual geometry with separate normal indices.
Existing C1 APIs retain their rejection contract. Holes, general loops, sewing
and closed rational solids remain incomplete. The estimate remains 15–25%,
toward 80%. See [crease-aware polygon display](docs/nurbs-polygon-crease.md).

## Editable browser rational polygon faces

The shared native/WASM polygon fixture now exposes an actual retained triangular
UV B-rep face and bounded interior display in three modes: weighted biquadratic,
C1 refinement and crossed C0 roof. Browser editing, exact-edge boundary overlays,
one-sided normals and rejected-edit preservation are verified alongside full
JSON parity and independent curve/surface/bound checks. A captured actual screen
documents the working UI. This is an open face, not a closed solid. General loops,
holes, sewing and closed rational solids remain incomplete. The estimate remains
15–25%, toward 80%. See [polygon demo](docs/nurbs-polygon-demo.md).

## Rectangular inner wires on convex rational faces

`NurbsPolygonHoledFace` retains exact clockwise inner wires and shared global
references inside a convex UV outer boundary, with independently guarded UV
clearance and explicit contact/overlap rejection. Convex cell partition before
triangulation preserves material boundaries, C0 normal identities and excluded
singularity sampling. Native independent bounds/area/topology/resource tests,
full native/WASM diagnostic parity and an editable browser opening are verified.
Nonrectangular holes, general trims, sewing and closed rational solids remain
incomplete. The long-term estimate remains 15–25%, toward 80%. See [convex
face openings](docs/nurbs-polygon-holes.md).

## First closed polynomial NURBS solid domain

`NurbsGraphSolid` retains an exact biquadratic roof, planar walls and base
as six NURBS faces with eight shared vertices and twelve shared edges.
Its scoped certificate checks the canonical geometry, pcurves and opposing
edge uses; exact volume comes from polynomial integration. A common bounded
display grid preserves closure across faces while keeping separate normals.
The editable native/WASM browser demo exercises actual closed geometry.
Arbitrary NURBS solid validation, sewing, placement, Booleans and STEP support
remain incomplete. The long-term estimate remains 15–25%, toward 80%.
See [polynomial graph solids](docs/nurbs-graph-solid.md).

## Rigid placement of polynomial NURBS solids

The scoped graph solid now transforms its exact source curves, surfaces and
shared vertices under a cumulative rigid placement, preserving topology,
pcurves and analytic volume. Rebuilding from local canonical controls avoids
accumulated control-net rounding. Checked world-coordinate allowances reject
unresolved physical or display requests. Nonidentity bounds are explicitly
conservative control-hull enclosures. Native/WASM and editable browser tests
cover actual placed geometry; arbitrary NURBS solid operations remain
incomplete. The estimate remains 15–25%, toward 80%.
See [graph placement](docs/nurbs-graph-placement.md).

## Closed NURBS graph solid restriction

`trimmed_uv` exactly restricts the source roof and constructs a matching
base and four ruled walls, including newly curved upper boundaries. Shared
edges and pcurves close the result; exact polynomial integration measures its
volume. Nested restrictions retain original source parameters and placement,
and independent bounds cover all six displayed faces. The browser edits the
retained UV rectangle through the same native/WASM kernel. This remains a
scoped rectangular solid restriction; general trims, sewing and curved
Booleans remain incomplete. The estimate remains 15–25%, toward 80%.
See [graph solid restriction](docs/nurbs-graph-trim.md).

## Source-axis plane partition of NURBS graph solids

`split_uv` now returns both closed material results, an exact ruled NURBS
section and its placed cutting plane. Child canonical geometry, shared
section source controls, opposite cut-face orientations and conserved
analytic volume are checked. Native/WASM and browser side selection exercise
trimmed, placed and signed-offset sources, with explicit contact rejection.
This supports source-axis planes only; arbitrary curved-solid partition,
general Booleans and sewing remain incomplete. The estimate remains
15–25%, toward 80%.
See [graph plane partition](docs/nurbs-graph-split.md).

## Closed NURBS graph solids with through openings

`NurbsGraphHoledSolid` now retains one exact rectangular through opening as
inner cap wires and four additional ruled cavity walls, with sixteen shared
vertices, twenty-four edges and ten faces. Stable positive-strip integration
measures the remaining volume without source-minus-tool cancellation.
Boundary-conforming material-only display shares rim nodes, retains inward
normals and bounds every face. Native/WASM and the browser edit actual
genus-one solids, including trimmed and placed sources. Arbitrary tools,
multiple/intersecting openings, general curved Booleans and NURBS interchange
remain incomplete. The estimate remains 15–25%, toward 80%.
See [graph through openings](docs/nurbs-graph-hole.md).
## Scoped NURBS graph-solid query milestone

Point classification now checks actual retained rational faces, including the
single through opening, with Euclidean distance bounds and explicit unresolved
errors. Restricted/placed graph solids share the same native/WASM query, with
browser point inputs and a world-coordinate marker. This is groundwork for
curved-solid queries; it does not implement general NURBS classification or
Boolean intersections. The long-term estimate remains **15–25%**, toward the
**80%** target. See [query contract](docs/nurbs-graph-classification.md).

## Scoped source-vertical material sections

Retained graph solids now return physical-height material intervals, cap
crossing events with actual normals and exact finite line segments at original
source UV coordinates. Restricted/placed sources and through-opening voids
share native/WASM and browser queries. Euclidean wall contact, invalid source
certificates and unresolved precision return explicit errors. Arbitrary line
directions and general curved-solid intersections remain unsupported. The
estimate remains **15–25%**, toward **80%**.
See [source-vertical sections](docs/nurbs-graph-section.md).

## Scoped NURBS graph-solid STEP export

Graph solids, including restricted/placed sources and one rectangular through
opening, now export actual polynomial spline bases, shared B-rep topology and
both retained pcurve uses as AP214 in explicit mm units. Independent tests
evaluate the serialized bases and topology; an optional external reader verifies
four documents in mm/metre output units, face counts, closed imported meshes,
bounded approximate volumes and preserved hole crossings. Native/WASM and
accepted-model browser downloads share the writer. Non-unit rational geometry,
arbitrary NURBS shells and graph-solid STEP import remain unsupported. The
long-term estimate remains **15–25%**, toward **80%**.
See [scoped NURBS STEP export](docs/nurbs-graph-step.md).

## Strict full-graph NURBS STEP import

One unplaced, full-domain six-face polynomial graph solid can now be read from
actual STEP spline controls, knots, pcurves and shared topology. Exact canonical
recognition retains the parsed geometry, reindexes identities and rejects even
sub-tolerance noncanonical edits without snapping. Independent tests cover
mm/metre conversion, entity/record/face ordering, cyclic wire starts and invalid
geometry/units/references/resource limits. Native/WASM and browser import and
re-export share the Rust parser, preserving accepted models on rejected input.
The later holed-import milestone below adds one rectangular opening; placed,
restricted and general rational imports remain unsupported.
The long-term estimate remains **15–25%**, toward **80%**.
See [strict graph STEP import](docs/nurbs-graph-step-import.md).

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
See [holed graph STEP import](docs/nurbs-graph-holed-step-import.md).

## Common checked graph STEP entry point

One bounded parse now selects plain or single-opening recognition from actual
shell topology and returns a typed `ImportedNurbsGraph` certificate. Shared
validation, volume, bounds, display, point classification, vertical section
and STEP export dispatch preserve the original exact geometry and rejection
policy. Native and safe WASM entry points accept both supported kinds without
caller-side STEP inspection. This improves integration, not geometric coverage:
placed/restricted imports and general shells remain unsupported.
The estimate remains **15–25%**, toward **80%**.
See [common graph import](docs/nurbs-graph-step-auto-import.md).

## Scoped graph uniform-density mass properties

Certified graph solids and single-opening graph solids now expose volume and
world centroid from polynomial column moments. Retained UV rectangles and
four positive material strips support trimmed and off-centre holed parts without
mesh integration or near-equal volume subtraction. Rigid placement preserves
volume and transforms the centroid. Browser and native/WASM JSON show accepted
properties; generic rational-shell mass, mixed density, inertia and surface
area remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [graph mass properties](docs/nurbs-graph-mass-properties.md).

## Scoped graph centroidal inertia

Polynomial graph solids and rectangular-through-opening solids now expose
centroidal inertia in world axes at uniform unit density (geometric mm⁵).
Direct central column moments and positive material strips preserve trims,
signed roof offsets and rigid placement without mesh or world-moment
subtraction. Browser diagonal inspection and complete native/WASM tensors
record explicit units/reference; unresolved numeric results remain errors.
General-shell mass, mixed density, principal-axis solvers and surface area
remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [graph inertia](docs/nurbs-graph-inertia.md).

## Exact affine-UV graph roof sections

Roof queries now preserve exact degree-four NURBS curves along arbitrary finite
affine source-UV paths, including restrictions, placement and rectangular-hole
material gaps. Five algebraic Bernstein controls, unit weights and global path
knots retain the roof pcurve after clipping; private certificates reject public
mutation. Contact/conditioning cases remain explicit errors. Native/WASM reports
and browser overlays use exact curves plus bounded display tessellation. These
roof queries return curves only. The later source-vertical oblique
partition milestone adds closed parts and finite sections for plain/convex
graph stock; arbitrary shell/plane intersections remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [roof sections](docs/nurbs-graph-roof-section.md).

## Closed convex polygon graph solids

Polynomial graph solids now retain strictly convex 3–16-corner source-UV
boundaries as closed B-reps with shared exact lifted edges and ruled NURBS
walls. Trimmed and placed sources preserve original UV pcurves. Positive
triangle integration gives volume and world centroid; conforming bounded
display evaluates actual retained faces. Native/WASM and an editable browser
demo verify accepted solids and explicit invalid-input rejection. Concave
boundaries and general curved Booleans remain unsupported. Strict polygon
STEP import now supports the unplaced full-source-UV plain profile. The estimate remains **15–25%**, toward **80%**.
See [closed polygon graph solids](docs/nurbs-graph-polygon.md).

## Source-vertical oblique graph partitions

A directed affine source-UV line now partitions rectangular or convex polygon
graph stock into two closed NURBS solids. Crossings are computed once; opposite
cut walls share exact reversed control nets and weights. A retained finite
NURBS section, placed world plane, strict source/result certificates and
positive volume integration verify the operation. Native/WASM and browser
side selection preserve accepted models on contact/conditioning failures.
Non-source-vertical planes, concave/holed stock and general curved Boolean
operations remain unsupported. The estimate stays **15–25%**, toward **80%**.
See [oblique graph partitions](docs/nurbs-graph-polygon-split.md).

## Convex polygon through openings

Closed polynomial graph solids now retain one strictly contained convex polygon
opening, annular cap wires and shared inward NURBS walls. Positive material-domain
quadrature gives volume and centroid; checked conforming display preserves the
void. Native/WASM and the editable browser demo validate topology, precision
and rejected inputs. Multiple openings, general curved Booleans, polygon-hole
STEP import and inertia remain unsupported. The estimate
remains **15–25%**, toward **80%**.
See [polygon through openings](docs/nurbs-graph-polygon-hole.md).

## Polygon graph point classification

Typed convex polygon and single-opening graph solids now classify world points
against the Euclidean boundary band of actual retained faces. Checked Bernstein
lower bounds and material-only surface witnesses exclude removed roof/base
regions and finite-wall extensions. Native/WASM queries and browser point
markers preserve the last accepted model/results after invalid edits. Precision
and subdivision limits return explicit errors. General NURBS classification,
multiple openings and curved Booleans remain unsupported. The estimate remains
**15–25%**, toward **80%**.
See [polygon graph point classification](docs/nurbs-graph-polygon-classification.md).

## Polygon graph STEP export

Convex polygon and single-opening polynomial graph solids now export actual
closed B-reps as AP214 STEP. Rational complex spline records preserve retained
near-unit weights, parameter maps, annular bounds and shared topology.
Native/WASM and browser downloads use accepted modeling inputs. Polygon STEP
import, multiple openings and general curved Booleans remain unsupported.
The estimate remains **15–25%**, toward **80%**.
See [polygon graph STEP export](docs/nurbs-graph-polygon-step.md).

## JSON binary64 model reproducibility

Native and WASM now enable precise JSON float round trips. Concrete one-ULP
parser changes are fixed; finite serialized coordinate bits, reconstructed
16-corner solids and STEP bytes are preserved. Tests retain original
trigonometric inputs rather than replacing them with dyadic fixtures. Browser
workflow import/download/autosave checks cover actual model coordinates.
Geometry limitations remain unchanged; the estimate stays **15–25%**, toward
**80%**. See [JSON binary64 preservation](docs/json-float-roundtrip.md).

## Guarded polygon footprint exclusion

Polygon point queries can now prove `Outside` from physical support-line
distances before curved-face subdivision. Deep opening points and separated
outer points avoid unnecessary refinement; near-wall queries retain the actual
Euclidean boundary search. Canonical validation and source/world precision
guards remain mandatory. The placed removed-roof example now resolves at
1e-8 mm without relaxing its requested band. Very small unresolved bands and
arbitrary NURBS bodies still return explicit errors. The estimate remains
**15–25%**, toward **80%**. See [point classification](docs/nurbs-graph-polygon-classification.md).

## Polygon graph centroidal inertia

Convex polygon and single-opening polynomial graph parts now compute full
uniform-density centroidal world inertia. Positive Duffy/Gauss7 material
integration avoids subtracting nearly equal stock and cavity tensors; local
centering avoids world-origin cancellation. Native/WASM reports and Web demos
show the checked tensor. Typed parts with suitable explicit tolerances verify
actual moment overflow; fixed-tolerance public demos reject oversized geometry
before evaluation. Optional report errors remain separate from shape data. Generic rational-shell inertia, variable density and principal axes
remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [polygon inertia](docs/nurbs-graph-polygon-inertia.md).

## Multiple convex polygon graph openings

A new typed polynomial graph family supports 1–4 strictly convex, separated
through openings with at most 64 total corners. Actual cap inner wires and
inward walls retain closed genus-g B-reps. Positive material integration
provides volume, centroid and world inertia; bounded display follows actual
faces and excludes every opening. Exact collinear input-vertex edge splits
handle aligned holes without nudging geometry. Native and WASM share an
editable Web demo and checked count-based numeric transport. Pair contact,
overlap/nesting and unresolved precision/resource requests are rejected.
STEP export now retains all cap loops, actual rational weights and shared
topology. Native/WASM export builds the model without display meshing, and
the browser downloads its last accepted part. Point classification now checks
the retained material outside every opening; STEP import remains unsupported.
The estimate remains **15–25%**, toward **80%**.
See [multiple polygon openings](docs/nurbs-graph-polygon-multi-hole.md).

## STEP export for multiple polygon graph openings

The existing 1–4-opening, 64-total-corner family now exports its validated
closed B-rep as AP214 in millimeters. Every cap inner wire, inward wall, shared
edge and incident pcurve is retained, including near-unit rational weights.
Model-only native/WASM export remains independent of display mesh resources;
browser downloads use the accepted model. Modeling admission conditions are
unchanged. Multiple-opening point classification is now available; STEP import
and general curved Booleans remain unsupported. The estimate remains **15–25%**, toward **80%**.
See [multi-opening STEP export](docs/nurbs-graph-polygon-multi-hole-step.md).

## Point classification for multiple polygon graph openings

The typed 1–4-opening, 64-total-corner family now classifies world points
through a Euclidean band around actual retained cap material and finite walls.
Positive UV material regions exclude every removed cap area; guarded supporting
planes resolve deep void points in any opening before curved-face search.
Source/world arithmetic and depth/visit/work budgets are unchanged, with
explicit unresolved errors. Model-only native/WASM queries share checked
transport with four appended query values, at most 151 finite values. The
browser queries its accepted model and preserves results/markers after failed
edits or queries, alongside accepted-model STEP downloads. Multiple-opening
STEP import and generic NURBS queries remain unsupported. The estimate remains
**15–25%**, toward **80%**. See [multi-opening point classification](docs/nurbs-graph-polygon-multi-hole-classification.md).

## Exact holed STEP recognition prefilter

The strict identity/full-source-UV graph importer with one rectangular opening
now compares each bounded coefficient candidate’s actual roof basis before
building a complete B-rep. Matching candidates still require full actual
geometry reindexing and canonical validation; acceptance conditions, units,
parser/search limits and errors remain unchanged. Import-only native and WASM
measurements separate recognition cost from export and display work. The local
three-sample WASM finish medians were 7.6–23.8% lower; native medians were mixed
in the shared environment. These measurements do not establish a universal or
whole-browser speedup. This adds
no dependency and does not broaden this rectangular-opening importer to polygon
or multiple openings or general NURBS shells. The estimate remains **15–25%**, toward **80%**.
See [holed import recognition cost](docs/nurbs-graph-holed-step-import-performance.md).

## Strict plain polygon graph STEP import

A separate typed AP214 reader now retains actual unplaced full-source-UV
3–16-corner convex polygon graph B-reps. Positive rational weights and 3D
controls/knots are preserved. Raw affine line origin, direction ratios and
magnitude must exactly match the writer’s canonical preimage before a pcurve
representative is restored; rounded ratio products do not establish acceptance.
Full canonical geometry/topology validation follows, with bounded coefficient
recovery and stable re-export after canonical ordering. The 1 MiB parser and
finite resource limits remain. Native/WASM and browser workflows display the
checked imported body and preserve accepted state after rejection. Generic and
existing automatic graph import APIs retain their scopes; polygon holes, pose,
trim and general rational shells remain unsupported. The estimate remains
**15–25%**, toward **80%**. See [strict polygon STEP import](docs/nurbs-graph-polygon-step-import.md).

## Rational curved-boundary foundation

Single-Bezier rational UV curves now compose with single-Bezier rational
surface patches through original homogeneous Bernstein algebra, retaining
actual weights and the curve parameter interval. Checked rational pcurves
provide matching UV evaluation; unsupported analytic trim/area operations
reject this boundary type. A degree-eight roof-circle path demo retains
actual geometry and leaves the stock solid unchanged. The separate checked
circular-bore milestone below builds on this foundation. The estimate remains
**15–25%**, toward **80%**.
See [rational roof-circle paths](docs/nurbs-graph-rational-roof-circle.md).

## Closed circular graph bores

A dedicated checked graph operation now removes one strictly contained
source-XY circular through column, preserving rational circular cap trims,
shared degree-eight boundaries and four inward ruled walls in a closed B-rep.
Volume, centroid and full world inertia use real-exact polynomial quadrature
with cancellation and scaling guards. Display samples the actual B-rep;
its bounds include circular trim approximation rather than claiming exact
mesh coverage of the circle. General curved Booleans, blind graph bores,
interacting tools, STEP import and editable document integration remain
subsequent work. The estimate remains **15–25%**, toward **80%**.
See [circular graph bores](docs/nurbs-graph-circular-hole.md).

## Circular-bore STEP export

Dedicated AP214 export retains the actual rational rims, ruled spline walls,
two-dimensional rational cap pcurves and both uses of every shared edge.
Model-only export does not depend on display tessellation. The web demo exports
its accepted model; rejected edits preserve that state. Existing affine STEP
output remains unchanged. Circular-bore STEP import, general curved Booleans
and editable operation-document integration remain unsupported. The estimate
remains **15–25%**, toward **80%**.

## Circular-bore point classification

Dedicated queries classify the actual closed circular-bore solid with a checked
Euclidean boundary tolerance. Removed cap regions never provide boundary
witnesses; finite inner walls and circular rims remain searchable. Canonical
validation, arithmetic guards and finite subdivision budgets reject unresolved
cases explicitly. Browser queries use the accepted model and preserve prior
results and markers after rejection. General curved-solid classification and
editable operation-document integration remain subsequent work. The estimate
remains **15–25%**, toward **80%**.
See [circular graph bores](docs/nurbs-graph-circular-hole.md).

## Editable graph operation documents

A dedicated versioned document now connects graph stock, absolute UV selection,
rigid placement and an optional final circular through bore to typed replay,
actual incremental prefix reuse and transactional display validation. Browser
Undo/Redo replays accepted documents; local save/restore validates intent in
Rust. Accepted-model point queries and STEP export share the retained typed
shape. Unsupported histories and failed geometry/display edits preserve the
last accepted state. Complex accepted histories remain editable as JSON rather
than being flattened by the simple form. General curved modeling, circular-bore
STEP import and arbitrary operation graphs remain future work. The estimate
remains **15–25%**, toward **80%**. See [editable graph workflow](docs/graph-workflow.md).

## Isolated planar edge chamfer

An equal-setback chamfer now splits a strictly convex planar straight-edge
solid into actual retained and removed closed B-reps, with a shared planar
bevel and exact STEP export. The browser displays accepted geometry and
preserves it after rejected edits. Curved, interacting and general chamfers
and fillets remain unsupported. The estimate remains **15–25%**, toward
**80%**. See [planar edge chamfer](docs/edge-chamfer.md).

## Transverse multiple planar chamfers

Several original-edge bevel planes now intersect actual retained convex planar
B-reps, including resolved unequal adjacent bevels. Removed-piece interiors
are sequentially disjoint; returned bevels are final trimmed faces. Existing
vertex contacts, notably equal adjacent/all-edge box bevels, remain explicitly
unsupported, along with curved/general chamfers and fillets. The estimate
remains **15–25%**, toward **80%**. See [multiple chamfers](docs/edge-chamfer-multi.md).

## Guarded vertex-contact planar chamfers

A dedicated convex planar chamfer entry point now handles resolved intermediate
vertex contacts, including all twelve equal-setback box edges. Actual closed
B-reps retain source vertex positions and final bevel faces; displacement and
intersection conditioning guards reject unresolved cases. The original
transverse APIs retain their narrower domain. Curved/general chamfers and
fillets remain future work. The estimate remains **15–25%**, toward **80%**.
See [contact-capable chamfers](docs/edge-chamfer-contact.md).

## Parallel box-edge circular fillets

A scoped fillet operation now recognizes actual orthogonal box B-reps and
rounds one to four parallel original edges with tangent circular arcs and
cylindrical faces. The retained closed solid and direct analytic removed
volume are available; cusp-bearing removed solids are not returned. Separate
bounded-analytic STEP export retains shared analytic geometry and pcurves.
General fillets and mixed-axis blends remain future work. The dedicated
bounded circular STEP reader is documented below. The estimate remains **15–25%**, toward **80%**.
See [parallel box-edge fillets](docs/edge-fillet.md).

## Bounded analytic STEP import

A separate opt-in reader retains actual lines, circles, bounded circular arcs,
planes, cylinders and both surface pcurves. Line/arc bodies require a checked
simple normal extrusion with matching caps and shared walls. Unit conversion,
resource limits and geometric conditioning are explicit. A native example and
WASM browser import/export workflow accompany the reader. General STEP,
freeform surfaces and arbitrary curved solids remain future work.
The estimate remains **15–25%**, toward **80%**.
See [bounded analytic STEP import](docs/step-bounded-import.md).

## Analytic STEP prisms with through openings

The bounded analytic reader now retains actual line/arc normal prisms with
one outer and up to sixteen disjoint inner cap wires. All translated cap
boundaries, shared walls and wire roles are checked without replacing geometry.
A rectangular-plus-circular openings sample demonstrates actual STEP
export/import, inward walls, genus-two closure and bounded display.
Contact, nesting, outside and unresolved precision reject; general curved
Booleans and arbitrary STEP solids remain future work. The estimate remains
**15–25%**, toward **80%**. See [analytic STEP openings](docs/step-bounded-import.md).

## Normal circular bores in line/arc prisms

A dedicated normal through-bore operation now cuts certified line/arc stock,
including rounded boxes and sources with disjoint existing openings. Both
retained and removed results are closed analytic B-reps, with source-curve
retention, inward hole walls and independent volume/conservation checks.
The native/WASM demo composes actual box fillets with the bore and exports
both results. General curved Booleans, oblique axes and contacting cuts remain
future work. The estimate remains **15–25%**, toward **80%**.
See [normal prism bores](docs/arc-line-prism-bore.md).

## Editable rounded stock and normal bores

The existing operation document now includes actual rounded-box stock followed
by up to 16 disjoint normal through bores. Corner-radius and bore edits use
unchanged exact B-rep prefixes, atomic rejection, Undo/Redo, saved documents
and bounded analytic STEP export in the existing editor. Blind rounded-stock
cuts and general curved Booleans remain unsupported. See [rounded workflow](docs/workflow-rounded.md).
This integrates existing verified analytic operations; the long-term estimate
remains 15–25% toward the standing 80% target.

## Editable line/arc normal profiles

The existing operation document now accepts custom normal line/signed-quarter-arc
profiles, including noncentered XY coordinates and initial disjoint openings.
Additional through bores share the actual source region and combined
16-opening/128-segment budget. Native/WASM sessions, prefix caching, Undo/Redo,
saved documents and analytic STEP exports use the existing workflow.
Blind/skew/arbitrary-plane stock and general curved Booleans remain outside
this document operation. See [line/arc workflow](docs/workflow-arc-line.md).
The long-term estimate remains 15–25% toward the standing 80% target.

## Axial-plane line/arc prism partition

Certified normal analytic stock now partitions into both actual closed sides
of one axial-plane cut. Original line/subarc coverage, opposing section faces,
uncut opening ownership and child-scale volume checks accompany construction.
Meaningfully oblique planes, hole crossings, contacts, vertex/tangent passage
and disconnected sections remain unsupported. This is broader analytic Boolean
infrastructure, not general curved Boolean completion or a new workflow node.
See [analytic prism partition](docs/arc-line-prism-split.md). The long-term
estimate remains 15–25% toward the standing 80% target.

## Editable analytic-stock plane cuts

Plane partition is now an actual selected-side node in rounded/line-arc
operation history, with subsequent cuts and through bores evaluated on the
retained body. Cut offset/angle/side edits, operation removal/relinking, prefix
reuse, atomic rejection, Undo/Redo, saved intent and accepted STEP export share
the existing native/WASM workflow. General operation graphs remain unsupported. Box/normal Polygon cut
nodes are implemented in the later milestone below. Resolved hole crossings and all-line child
continuation are implemented in the later milestones below. See [editable plane cuts](docs/workflow-plane-split.md).
The long-term estimate remains 15–25% toward the standing 80% target.

## Analytic cuts across openings and disconnected components

The additive normal line/arc component partition cuts across resolved openings
and returns all closed bodies on both sides with every actual material-interval
section. Original curve coverage, opposed cut-wall pairing, per-child area
conditioning and child-scale volume checks accompany the construction.
Editable cut histories accept this result when the selected side contains one
connected body; multiple selected bodies reject atomically. Native/WASM and
browser demos use the actual partition and independent STEP exports.
Contacts, vertex/tangent passage, meaningful skew and general curved Booleans
remain unsupported. See [component partitions](docs/arc-line-prism-split-components.md).
The long-term estimate remains 15–25% toward the standing 80% target.

## Continued machining of all-line cut children

Explicit-axis normal-prism bore and component-partition APIs now accept actual
all-line prisms as well as certified line/arc prisms. Editable rounded/line-arc
stock histories can continue cutting and drilling after all curved stock
boundaries have been removed. Actual cap-family selection, normal-translation
proofs, original curve coverage and existing precision guards accompany the
construction; skew and ambiguous cap families reject. Existing Arc-specific
APIs retain their stricter domains. See [normal-prism continuation](docs/workflow-plain-continuation.md).
General curved Booleans and blind machining on these histories remain future
work. The estimate remains 15–25% toward the standing 80% target.

## Box and normal polygon cut histories

Actual selected-side plane cuts now extend to Box and normal-Z Polygon
roots, with through bores before and after cuts. Initial polygon holes,
concavity, resolved hole crossings and uniquely connected selection share
the normal-prism certificates. First/last-cut transitions reuse only the
actual stock prefix because prior bores switch between legacy full circles
and certified quarter arcs. Exact native/WASM histories and STEP export
remain reproducible. No-cut through/blind behavior is preserved; skew and
blind machining in cut histories explicitly reject. See [normal stock cut
histories](docs/workflow-planar-cuts.md). General curved Booleans remain future
work. The estimate remains 15–25% toward the standing 80% target.

## Inspection across model scales

The shared display-only viewer now normalizes actual mesh coordinates and fits
a bounding sphere for viewport aspect, relative zoom and safe clipping. Small
cut parts and large models remain inspectable without fixed-unit camera limits.
CAD geometry/STEP coordinates stay unchanged; marker normalization follows
accepted mesh scale. Independent projection tests accompany actual browser
inspection. See [scale-aware display](docs/viewer-scale.md). This
addresses end-to-end inspection usability; broad CAD coverage remains
15–25% toward the standing 80% target.

## Flat-bottom pockets in certified normal stock

A standalone exact blind-bore API now retains actual curved/all-line source
stock and adds shared quarter-cylinder walls and a planar floor. Positive or
negative cap entry follows an explicit signed normal axis; disjoint source
through openings, rigid placement and all-line cut children remain admissible.
Actual kept/removed B-reps, direct removed volume, browser inspection and
analytic STEP export accompany the operation. Continued machining of blind
bodies and workflow activation remain future work. A later single-cavity STEP
certificate adds verified re-import below.
See [normal blind bores](docs/normal-prism-blind-bore.md). The long-term
estimate remains 15–25% toward the standing 80% target.

## Verified single-cavity STEP re-import

The opt-in bounded analytic reader now admits one resolved four-quarter blind
pocket in certified normal stock. Exclusive cavity topology is removed only
in a proof clone; the actual restored source and strict blind-construction
witness certify the imported cavity, including analytic whole-curve/pcurve
identity and shared orientation. Original imported geometry remains unchanged.
Both entry caps, rigid placement and existing disjoint through openings are
covered. Later milestones add multiple pockets and continued machining through
the separate certified API; general STEP remains unsupported. See [single-cavity STEP](docs/normal-prism-blind-step.md).
The long-term estimate remains 15–25% toward the standing 80% target.

## Finite segments through bored oblique children

Actual retained annular children now support mesh-free finite segment queries.
Independent clipping preserves both material intervals across a hole; cut roots
in the void produce no cap hit. Actual restricted-face UV and oriented normals
are checked, with explicit errors for contacts and unresolved precision.
See [finite segment scope](docs/nurbs-frustum-bored-oblique-segment.md).
Line/ray queries for this child type remain future work. The long-term estimate
remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, 21 relevant native tests and the
WASM build passed. Combined native/WASM segment/workflow/mass/point checks
passed. The entire browser suite passed on its second run; the first run
failed an existing line-probe exact PNG comparison before the new block.
A targeted raw-buffer/camera/PNG diagnostic passed without reproducing the
cause; the original equality guard remains intact and no fix is claimed.
The actual final model screenshot was inspected.

## Original-parameter lines and rays through bored oblique children

Actual retained annular children now share checked finite cropping and original
nonunit parameter mapping with other retained frusta. All material intervals
are preserved; rays classify the actual origin and clip every interval without
a synthetic boundary hit at zero. Existing source/world precision and contact
guards remain unchanged. Repeated cuts and general curved Booleans remain
unsupported. See [retained line/ray scope](docs/nurbs-frustum-retained-line.md).
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, 18 relevant native tests and WASM
build passed. The entire new line/ray block plus prior segment/workflow/mass/point
native/WASM checks passed. The entire browser suite passed on its first run;
actual ray interval `[2,7]` and retained annular model screenshot were inspected.

## Exact cardinal bored-frustum STEP re-import

A separate bounded millimetre reader admits one canonical coaxial through bore
in translated right-handed signed-cardinal frames. Actual 16V/24E/10F B-rep,
rational coefficients and both annular cap wires are retained after a complete
certificate. General rotations, oblique children, other units and generic STEP
remain unsupported. See [exact bored STEP scope](docs/nurbs-frustum-bored-step-import.md).
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, 23 relevant native tests and WASM
build passed. The entire new importer and all three previous frustum importer
WASM blocks passed with native report/STEP parity and strict error recovery.
The entire new browser regression passed on its first run; the actual imported
annular child screenshot was inspected. CI includes the new browser regression.

## Full regression verification after annular STEP admission

On the exact pushed implementation commit `ab716f4`, the entire locked native
suite passed: 307 suites, 1,187 tests, zero failures and zero ignored tests,
including documentation tests. The entire WASM runtime script also completed
with exit zero, including prior primitive/STEP/workflow/predicate checks and
all new annular query/import blocks. No source was changed during these checks.
This validates regression coverage; the standing broad CAD estimate remains
15–25% and does not imply general curved Booleans or STEP support.

## Repeated exact-basis parallel frustum cuts

New typed plain and bored slabs retain the original source and both actual
cap boundaries. Strict exact stored plane bases and full-source clearance
guards permit repeated selected-child cutting, with actual closed B-reps,
positive checked volumes, conforming display and exact STEP export. Metrics
and point/segment/line/ray queries of slab children remain explicitly unsupported;
skew recuts and generic curved Booleans remain future work.
See [parallel slab scope](docs/nurbs-frustum-parallel-slabs.md).
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, 29 relevant native tests and the
WASM build passed; seven workflow tests also passed after the final scope
update. Combined new parallel-cut and previous oblique workflow WASM blocks
passed with native JSON/STEP parity. The entire browser suite exited zero,
including repeated plain/annular cuts and all existing rollback checks.
Browser retries exposed a collapsed-control test setup error and screenshot
composition differences with identical raw WebGL pixels; test-only opaque
comparison backgrounds now survive reloads, while all exact PNG assertions
remain. The production background was restored before capturing and inspecting
the actual two-cap annular slab (volume 2911.6631285677404 mm³).

## Analytic mass properties of retained parallel slabs

Plain and bored slab children now expose checked analytic centroid and full
world-space centroidal inertia. One-boundary children reuse their original
oblique moments; two-boundary children subtract cumulative local centered
moments, and annular children subtract the inner slab in the same source frame.
Cancellation and rounding allowances propagate through both stages. Geometry's
stable volume remains authoritative, and unresolved metric conditioning returns
explicit Unsupported while preserving actual B-rep, volume, display and STEP.
Point/segment/line/ray queries of slabs and skew recuts remain unsupported.
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: final fmt, strict all-target Clippy, seven relevant native suites
(18 tests) and WASM build passed. New cylinder/annular all-component tensor
oracles, posed/scaled repeats, tapered conservation and precision refusal tests
passed. The combined new mass and previous parallel/oblique WASM blocks passed
with complete native JSON/STEP parity; prior bored-oblique workflow output is
byte-identical after the shared helper extraction. The entire browser suite
passed on its first run, including optional metric refusal/STEP retention and
all exact image/raw-frame rollback checks. The actual middle-slab screenshot
with centroid and inertia was inspected. An earlier baseline comparison shared
a build output directory and contaminated cached native artifacts; that result
was discarded and current-source builds/checks were repeated. Existing display
tests were migrated from obsolete unavailable-metric expectations to exact
native centroid/tensor parity without changing their geometry assertions.

## Finite point queries of retained parallel slabs

Plain and bored slab children now classify points against their actual retained
material. The original source's Euclidean boundary policy is applied first;
both present cuts provide subset distance bounds. Boundary success requires a
retained source witness or an orthogonal cut-plane foot inside the finite disk
or annulus and strictly retained by the opposite cut. Ambiguous shared rims,
unresolved finite-face witnesses and world precision remain explicit
Unsupported. Missing original caps are not duplicated as cut boundaries.
Segment/line/ray slab queries and skew recuts remain unsupported.
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, ten relevant native suites (24 tests)
and WASM build passed. Independent finite annular/cylindrical distances,
posed/scaled repeated cuts, source-relative guards and a steep thin slab's true
finite-cap Boundary were checked; overlapping tolerance bands alone do not
invalidate a finite-face witness. Combined new point, previous mass/parallel
and ordinary/bored workflow WASM blocks passed with native query and STEP/state
parity. The entire browser suite passed on its first run with all exact PNG,
raw-frame, camera and accepted-STEP rollback checks. A stale capability sentence
was corrected before the final actual screenshot capture.
The final styled production-background capture passed its geometry, metric and
point-query assertions; the inspected image shows `[6,0,6]` classified Inside
for the actual middle annular slab. The temporary capture helper was removed.

## Finite segments through actual retained slabs

Plain slab children now expose zero or one material interval, and bored slabs
zero, one or two, using the original segment parameter. Checked original-source
intervals are clipped separately against both actual cut halfspaces. Only final
material endpoints become boundary hits, preventing removed earlier cut roots
or void crossings from creating phantom cap hits. Actual planar UV and ruled
wall parameters use both retained rational rim rows; face evaluation, oriented
crossing, midpoint material and endpoint correspondence are verified. Contact,
coincident endpoints, unresolved slivers and insufficient physical precision
remain explicit errors. Line/ray slab queries remain unsupported.
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: final fmt, strict all-target Clippy, nine relevant native suites
(22 tests) and WASM build passed. Independent cap clipping, void separation,
actual UV/oriented-normal, reversed/posed/scaled and no-phantom-hit checks passed.
Combined new segment and all prior point/mass/parallel/ordinary/bored workflows
passed with native reports and accepted STEP/cache/history parity. The first
new WASM fixture placed endpoints on original source caps and was correctly
refused; corrected outside endpoints passed, and original-cap refusal remains
an explicit test and documented limit. The entire browser suite passed on its
first run; all exact PNG/raw-frame rollback guards remained. The inspected
actual middle-slab screenshot shows four hits and material intervals
`[0.25,0.4]` and `[0.6,0.833333…]`, alongside its Inside point and mass properties.

## Original-parameter line and ray queries of retained slabs

Plain and bored slabs now dispatch into the existing checked retained-body
line/ray engine, using their actual B-rep, point classification and finite
segment intersection. Original canonical sources provide conservative line
enclosures and full-source precision budgets; original nonunit line parameters
are retained. All material intervals are mapped and forward-ray clipped, with
actual ray-origin membership and face witnesses verified. Existing numerical
crop and parameter algorithms are unchanged; tangent, boundary-origin, rim,
remote-anchor and unresolved precision cases remain explicit errors.
The long-term estimate remains 15–25% toward the standing 80% target.

Validation: fmt, strict all-target Clippy, ten relevant native suites (22 tests)
and WASM build passed. Independent actual cap/wall queries, all annular ray
intervals, original nonunit/reversed parameters, pose/scales, UV/normals and
original-anchor/contact failures passed. The combined entire new line/ray and
all previous slab/ordinary/bored workflows passed with native reports and
unchanged accepted STEP/cache/history. The entire browser suite passed on its
first run with all strict PNG/raw-frame rollback guards. The inspected actual
middle-slab screenshot shows its Inside point, cap-clipped finite segment and
hole-origin forward-ray interval `[2,20/3]`. Stale blanket recut limitations
were corrected to distinguish supported parallel cuts from unsupported skew.

## Full regression verification after retained parallel queries

On exact implementation commit `8a54e24`, the entire locked native suite passed:
316 suites, 1,207 tests, zero failures and zero ignored tests, including both
documentation tests. The entire WASM runtime script also completed with exit
zero, covering previous primitives, topology, predicates, STEP, workflows and
all new repeated parallel slab geometry, optional mass and point/segment/line/ray
blocks. Source stayed unchanged throughout both checks. This confirms regression
coverage without claiming generic curved Booleans or general STEP support.
The standing broad CAD estimate remains 15–25% toward the 80% target.

## Ordered nonparallel interior cuts of retained frusta

Separate plane-bounded plain and bored types now admit complete interior
sections with different normals. Whole-generator clearance uses a guarded
degree-four Bernstein product and an interior-minimum argument; crossing,
contact and unresolved precision remain explicit errors. Original rational
rims, actual two-row ruled walls, oriented annular caps and shared B-rep
incidence are retained. The existing parallel-cut API remains unchanged.
Actual-body display, checked optional mass, point/segment/line/ray queries and
STEP export are connected through the explicit `interior_plane_split` workflow
operation and an editable browser preset. No new dependency or OCCT code is used.

Validation: fmt, strict all-target Clippy, six relevant native suites (12 tests),
six existing slab regression suites (14 tests) and WASM build passed. The
combined entire new interior-plane workflow and all prior slab/ordinary/bored
workflow blocks passed on the final artifact, including independent disk and
annulus volume, all nine inertia components, dense actual pcurves, manifold
meshes, all query kinds, native STEP parity and tapered child conservation.
The entire browser workflow suite passed on its first run with all prior exact
PNG/raw-frame guards retained. The inspected actual middle-skew screenshot shows
volume `2882.687`, checked mass, an Inside point, four finite-segment hits across
two material intervals and the hole-origin forward-ray interval `[2,20/3]`.
This extends the bounded domain; general crossing-plane cuts, repeated bores,
generic curved Booleans and generic STEP import remain unsupported.
The broad estimate remains 15–25% toward the standing 80% target.

## Full regression verification after ordered interior planes

On exact implementation commit `6f199bf`, the entire locked native suite passed:
321 suites, 1,216 tests, zero failures and zero ignored tests, including both
documentation tests. The entire WASM runtime script also completed with exit
zero, including all existing primitives, NURBS, STEP, workflows and the new
ordered interior-plane geometry, optional mass and point/segment/line/ray blocks.
Source remained unchanged throughout both checks; subsequent retained-bore work
was isolated in a separate worktree. This is regression evidence for the bounded
implementation, not a claim of general curved Booleans or generic STEP support.
The standing broad estimate remains 15–25% toward the 80% target.

## Coaxial drilling and enlargement after retained cuts

Plain plane-bounded bodies now admit coaxial through drilling; bored
plane-bounded bodies admit strictly larger resolved radii. Actual external
geometry remains unchanged, and regenerated cylindrical rims attach to the
same caller cap planes. Explicit partition APIs return closed kept and removed
bodies: a finite cylinder for drilling, or a finite annulus for enlargement,
with opposing exact interface geometry. Admission retains the original full
source tolerance, world precision and minimum-radius bounds; equality, shrink,
contact and unresolved radial clearance are explicit errors.
Workflow drilling gains only this additional plain typed domain. A separate
`enlarge_coaxial_bore` operation accepts one explicitly selected bored
plane-bounded component. Existing canonical/older retained APIs keep their
previous domains. Actual display, checked optional mass, all four query kinds
and STEP export apply to the real kept and removed bodies.

Validation: final fmt, strict all-target Clippy, six relevant native suites
(eight tests) and isolated WASM build passed. Independent posed/scaled disk and
annulus moments, all nine tensor components, unchanged external coefficients,
closed genus-one topology, actual UV/normal witnesses, bounded closed displays,
kept/removed conservation and strict original-band rejection passed. Combined
new bore/enlargement and prior interior-plane WASM workflows passed with fresh
native replay, STEP, cache/history and failure recovery. The entire browser
workflow suite passed after reusing installed Playwright dependencies; the
initial isolated launch stopped before running tests because that package was
absent. The temporary dependency link was removed. All existing PNG/raw-frame
guards remained. The inspected actual enlarged-body screenshot shows volume
`2882.687`, checked mass, successful probes and the new hole-radius controls.
The broad estimate remains 15–25% toward the standing 80% target.

## Checked bore adapters for canonical and older retained families

Single-cut oblique and parallel-slab plain bodies now expose drilling and
closed kept/removed partitions through checked adapters. Their bored
counterparts expose strict enlargement and removed-annulus partitions.
The factories clone the actual original B-rep and retained bounds, validate
complete source association and coefficient identity, and apply whole-domain
ordering before admission. They never reconstruct an external lookalike.
Canonical bored frusta also expose strict enlargement and an independently
certified canonical removed annulus; that output stays in the canonical type.
All paths retain the original full-source tolerance and global minimum radius.
The workflow accepts these explicit typed domains and retains single-component
selection, atomic history and actual-body queries/export. Existing strict
parallel-cut and bounded STEP import APIs retain their previous domains.

Validation: final fmt, strict all-target Clippy, seven relevant native suites
(ten tests) and WASM build passed. Independent cylindrical disk/annulus moments
and all nine world inertia components, source/outer coefficients, exact old
inner ownership, actual UV/orientations, closed topology/display, scaled/posed
kept/removed conservation and immutable rejection/recovery passed. The complete
previous accepted workflow report (1,734,880 bytes, including point, segment,
ray and STEP) remained byte-identical after shared guard extraction. The entire
new adapter and prior interior/retained-bore WASM blocks passed with native
geometry/STEP, cache/history, manifold display and genuine domain failures.
The entire browser workflow suite passed on its first run with all earlier
strict PNG/raw-frame guards. The inspected actual parallel middle screenshot
shows volume `2911.663`, checked mass, its Inside point, two finite material
intervals and hole-origin forward ray `[2,20/3]`. Canonical, both oblique sides
and parallel typed hole/enlargement actions passed native STEP and history tests.
The broad estimate remains 15–25% toward the standing 80% target.


## Full horizontal-split regression checkpoint

Frozen source `bf45057` passed the complete locked native run: 350 suites,
1,263 tests, no failures or ignored tests, including two documentation tests.
The entire existing WASM test script also exited successfully against that
frozen artifact. These results cover the horizontal-split baseline before the
subsequent cached-child point-query milestone.
