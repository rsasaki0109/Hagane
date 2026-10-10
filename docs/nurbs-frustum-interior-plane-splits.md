# Strictly interior plane splits of retained frusta

`split_by_interior_plane(&Surface, GeometryTolerance)` is an additive operation
on oblique children, parallel slabs and the new plane-bounded types, with bored
counterparts. It returns actual closed `lower` and `upper` children of type
`NurbsFrustumPlaneBoundedSolid` or `NurbsBoredFrustumPlaneBoundedSolid`. Every
new plane section must be complete and strictly inside the current retained
material. Its normal can differ from the previous cut when the checked bounds
remain ordered. The existing `split_by_parallel_plane` and workflow `plane_split`
keep their stricter represented-parallel-plane requirement.

The original canonical source remains a checking witness. Each child retains
its actual lower and upper rational section boundaries, or an original source
cap at an uncut end. Actual degree `[2,1]` ruled walls use both positive rational
rim rows. Complete vertices, edges, cap loops, surfaces and orientations are
validated; no old slab or display mesh substitutes for the child solid. Canonical
children have 8 vertices, 12 edges and 6 faces. Bored children have 16 vertices,
24 edges and 10 faces, including actual annular caps and inward cavity walls.

The ordering check first bounds each plane's rational height denominator and
axis radius over the full source generator disk. A degree-four Bernstein product
then proves a strict axial clearance between lower and upper rational rim curves,
with arithmetic reserves. This rim bound extends through the disk: nonparallel
plane-height gradient vectors cannot produce an interior minimum of their
difference; parallel gradients reduce to a scalar range attained on the rim.
The same argument covers a coaxial annulus. Weight normalization is used only
for this proof; actual stored weights and control points remain unchanged.
These are checked engineering floating point bounds, not interval certificates.

`volume(policy)` uses guarded cumulative source-section differences. Analytic
`mass_properties(policy)` and `inertia_properties(policy)` reuse checked centered
moments in one original local frame. Bored material subtracts its actual inner
plane-bounded cylinder moments. Signed parallel-axis error allowances propagate
before final world tensor rotation. Unresolved cancellation or representability
can make optional metrics `Unsupported` while actual geometry, stable volume,
display and STEP remain available; reports never invent a centroid or tensor.

Display evaluates actual retained rational walls and uses topology-keyed shared
edge samples. Generator samples account for unequal endpoint-row weights.
Annular cap strips retain the opening with checked chord deviation. STEP export
writes the actual child B-rep. Point, segment, line and ray queries use the actual
retained faces and return explicit contact or precision errors; queries do not
perform a modeling cut.

The schema-1 workflow operation is `interior_plane_split` with `id`, `input`,
world `origin` and `normal`. A multi-component input requires explicit selection.
The result exposes both actual children; further nodes select a component rather
than implicitly taking the first or largest body. Failed geometry, display or
query requests preserve accepted state, cache, history and exports.

The [editable workflow fixture](nurbs-frustum-workflow-interior-example.json)
bores a tapered frustum, makes an oblique cut, then selects and recuts the lower
child with a differently oriented plane. Open the frustum workflow web demo and
choose its interior-plane preset to inspect the resulting middle component.

Planes that intersect an existing boundary, incomplete sections, insufficient
clearance, unresolved arithmetic and general curved Booleans remain unsupported.
This operation does not add generic STEP import or continued boring.

The implementation is original MIT OR Apache-2.0 code using rational sections,
positive homogeneous ruled patches and checked local moments. No OCCT code or
new dependency is used.
