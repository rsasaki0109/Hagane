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
subdivided faces are covered, with a native/WASM browser query demo. Curved
solid classification remains unsupported; see [classification](classification.md).

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

## Next: broaden analytic B-rep operations

- Extend frame-aware primitive/profile APIs and intersection routines.
- Extend explicit tolerance policies to more modeling operations, and add exact
  3D predicates and robust curve/surface intersections.
- Broaden mixed-profile placement/extrusion and support arbitrary cylindrical
  trim loops and general face splits.
- Implement scoped difference/union and broader retained-face selection using
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
