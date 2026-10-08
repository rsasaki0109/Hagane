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
the browser query page offers seven solid models, including bounded arc trims, rectangular partial cylinder walls and skew circular translation walls. General cylinder and NURBS trims remain unsupported. See [curved classification](curved-classification.md).

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

## Next: broaden analytic B-rep operations

- Extend frame-aware primitive/profile APIs and intersection routines.
- Extend explicit tolerance policies to more modeling operations, and add exact
  3D predicates beyond orientation and robust curve/surface intersections.
- Extend general trim clipping and subdivision to skew circular translation
  surfaces; broaden classification beyond rectangular trims and support arbitrary cylindrical
  trim loops and general face splits.
- Broaden retained-face selection and contact handling using
  planar arrangements; extend contact handling, surface/surface curves, intersection
  graphs and curved-face splits.
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
