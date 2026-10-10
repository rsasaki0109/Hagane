# Repeated parallel cuts of retained frustum material

`NurbsObliqueFrustumSolid::split_by_parallel_plane` and the bored counterpart
produce two actual typed slab children. Slab children support the same operation
again. The original finite canonical source is retained as a checking witness;
each child holds its actual lower and upper boundary sections, or the original
cap when that end remains uncut. Display and STEP use the retained B-rep.

The initial domain requires the new caller plane to have exactly the same
stored `u` and `v` basis as the previous cut. This is an explicit representation
restriction; nearly parallel directions are not snapped into support. The new
section must be complete and strictly inside the retained material on every
angular generator. Contact, cap crossing, tiny gaps, insufficient original-source
precision, nonfinite input and different plane bases return explicit errors.

The complete body certificate checks all actual vertices, edges, face surfaces,
wires and orientations. Ruled degree `[2,1]` walls use both actual rational rim
rows with positive weights. Shared straight generators carry their matching
rational surface parameter. Previously retained boundary curves remain exact.
Plain slabs have 8 vertices, 12 edges and 6 faces; bored slabs have 16 vertices,
24 edges, 10 faces and two annular cap wires. Cavity walls face into the hole.

Volume is a guarded difference of checked cumulative section volumes. Very
thin unresolved intervals are unsupported. Display uses actual surfaces and
a shared topology cache with a checked chord budget. Annular cap strips retain
the hole. STEP export writes the exact B-rep; no mesh CSG or reconstructed
substitute provides the cut.

The [working example](nurbs-frustum-workflow-parallel-example.json) cuts the
through-bored source at world z12 and then splits its lower child at world z6,
using the same normal `[0.2,0,1]`. The resulting volumes are approximately
3888.049 mm³ and 2911.663 mm³, summing to the selected parent
6799.712 mm³. The middle child retains two oblique caps and the through hole.

The editable workflow uses explicit component selection before a further
`plane_split`. The browser append action selects the displayed accepted component
and applies the entered world plane. Other components remain separate graph
outputs. Failed edits retain accepted geometry, camera, history and STEP.

Both `NurbsFrustumSlabSolid` and `NurbsBoredFrustumSlabSolid` provide
`mass_properties(policy)` and `inertia_properties(policy)`. Accepted metrics
report the actual world centroid and the full centroidal inertia tensor, with
unit density (`mm³`, `mm`, and `mm⁵`).

The calculation keeps checked centered moments in the original source-local
frame. A slab with two cut caps subtracts the lower cumulative section from the
upper cumulative section; a slab retaining an original cap uses the corresponding
single-cut moments. A bored slab then subtracts its inner cylindrical slab from
its outer slab in that same frame. Signed parallel-axis composition propagates
volume, mean and covariance error allowances through both subtractions before
rotating the resulting tensor into world axes. It does not subtract large world
raw moments or infer metrics from the display mesh.

The independently checked stable geometry volume remains authoritative. Moment
volume correspondence, finite arithmetic, centroid precision and positive
covariance guards can return `Unsupported` for unresolved cancellation, thin
material, near-uniform taper or extreme scale. These are engineering floating
point checks, not interval certificates. Workflow reports keep successful actual
geometry, display and STEP when optional metrics are unsupported, and expose
`mass_properties: {ok: false, error: ...}` without a fabricated centroid or tensor.
Other errors propagate.

Both slab types now provide `classify_point(Point3, GeometryTolerance)` against
their actual finite retained boundary. The original canonical or bored source
query supplies a conservative subset-distance check, including the through-hole
void. Inside admission also requires clearance from both retained cuts. A cut
cap can witness `Boundary` only when the Euclidean plane foot lies strictly in
the finite source material and is retained by the opposite cut. A source-face
boundary witness is admitted only with sufficient clearance from both cuts.
These checks do not combine separate plane and radial band labels into a false
rim distance.

The full relative tolerance uses the original finite source extent. Ambiguous
cut rims, boundary thresholds and unresolved world precision return
`Unsupported`. Overlapping cap bands can still produce `Boundary` when a finite
cap foot supplies a valid witness; overlap alone is not a refusal. Conflicting
bands are unresolved only when no admitted finite-face witness resolves them.
No nearest-point certificate is fabricated.
Workflow point probes use the actual accepted component and preserve its mesh,
metrics, STEP, cache and history. Segment, line and ray queries on slabs remain
explicitly unavailable.
Skew repeated cuts, general curved Booleans, continued boring and slab STEP
re-import remain unsupported. Existing canonical, bored and single-cut query
APIs keep their previous domains.

Original code is MIT OR Apache-2.0. This uses independently implemented rational
plane sections, positive homogeneous ruled patches, shared oriented topology and
cumulative section integration. No OCCT source or new dependency is used.
