# Axial partitions that retain a frustum through bore

`NurbsBoredFrustumSolid::split_axial_many(cuts, policy)` returns actual closed
`NurbsBoredFrustumPartitions.parts`, each a `NurbsBoredFrustumSolid`. Source-local
cut heights must be strictly increasing and strictly inside the original height;
1–16 cuts are accepted. Every interval must resolve the full original source
length band and numerical budget. No partial result is returned on failure.

The operation validates the original bored solid, partitions its actual source
with the checked axial restriction, then creates each child's finite coaxial
through bore at the unchanged radius. The inner cylinder is independently
partitioned at the same heights. Correspondence compares the complete spatial
curve and surface control nets with equal positive rational bases, so the
coefficient error bounds the whole retained inner wall. It does not depend on
sampling or exact byte equality of rounded constant-radius interpolation.
The check uses the original absolute tolerance reserve even with a relative
policy.

Every child has sixteen vertices, twenty-four shared edges and ten faces.
Both original or new end caps are annular, and inward cylindrical walls run
through the child's actual height and frame. The shell remains genus one;
filled caps or source-only solids are never substituted. Each child supports
analytic mass properties, bounded display, exact STEP export and actual finite
point/segment/line/ray queries.

Child volumes and first/second moments conserve the parent under checked
floating-point budgets. Independent tests include a nearly full-radius bore
with positive thin material, arbitrary poses/scales, opposed cap normals,
actual outer and inner seams, repeated child partitions and invalid cut lists.
The parent geometry remains immutable.

The workflow's existing `partition` node now accepts one explicitly selected
canonical or bored component. [This example](nurbs-frustum-workflow-bore-partition-example.json)
connects stock → coaxial bore → four axial parts. Select a part before further
partitioning its own local height. Edits retain unchanged graph prefixes and
Undo/Redo; all displayed outputs are the actual annular children with their own
STEP exports. Multipart recuts without Select, oblique partitions, repeated
bores and general frustum Booleans remain explicit unsupported operations.

Original code is MIT OR Apache-2.0. This composes Hagane's independently
implemented canonical restriction and coaxial cylinder construction; no OCCT
source or new dependency is used.
