# Finite segment queries on oblique frustum children

`NurbsObliqueFrustumSolid::intersect_segment(start, end, policy)` returns an
actual checked `NurbsFrustumSegmentIntersection`: boundary hits with kept face
IDs and surface UVs, and one optional material interval on the original segment
parameter range `[0,1]`. It queries the retained closed B-rep without a display
mesh or a modeling operation.

The retained body is the convex source frustum intersected with the halfspace
of its actual cut cap. The query first validates the actual child and original
endpoint/world/frame precision. It intersects the actual source and restricts
the source interval against the retained cap's outward plane normal. Finite
cut-face hits require an interior source witness; they do not use the infinite
plane as a replacement for the actual ellipse disk.

Original outer-wall hits keep their angular U parameter, but their source
height parameter cannot be reused on the restricted rational wall. At U, the
actual wall endpoints define a straight generator and its physical fraction q.
For bottom/top homogeneous row weights wb, wt, the kept V parameter is
`q*wb / ((1-q)*wt + q*wb)`. The implementation evaluates those weights from the
actual quadratic control net. Plane hits use the actual cap origin and basis.
Every returned witness must evaluate to the segment point within the original
absolute tolerance and its outward normal must agree with entering or exiting
the retained material.

The two complementary children partition resolved source material intervals
and share the actual cut hit. Reversing a segment reverses hit order and maps
parameters to `1-t`. General rigid frames share the same query implementation.

This initial query rejects boundary endpoints, tangencies, cut-rim contacts,
unresolved near-parallel crossings, tiny or unresolved material intervals,
near-coincident root ordering and insufficient world-coordinate precision.
Source-query boundary restrictions can conservatively reject an endpoint on
a removed original face. [Retained line/ray queries](nurbs-frustum-retained-line.md) are supported separately.

The [editable workflow](nurbs-frustum-workflow.md) uses the actual accepted
canonical, bored or oblique component in its read-only `segment` command.
Query failures preserve the accepted geometry, history, point result and STEP.

Independent native/WASM/browser tests compare complementary intervals, actual
nontrivial-U ruled-wall witnesses, cap witnesses, normal directions, reversed
segments, poses/scales and failure recovery. This is original MIT OR Apache-2.0
code based on elementary halfspace clipping and rational interpolation; no
OCCT source or new dependencies are used.
