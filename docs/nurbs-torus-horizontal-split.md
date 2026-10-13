# Horizontal torus plane splits

`NurbsTorusSolid::split_by_local_z(height, policy)` divides a regular ring torus
with a plane perpendicular to its represented axis. It returns
`NurbsTorusHorizontalSplit { lower, upper }`, containing two actual closed
`NurbsTorusHorizontalSolid` B-reps. Height is measured along the source frame's
local Z in model length units, so rotated and translated tori use the same API.

```rust
use hagane::{Frame3, GeometryTolerance, NurbsTorusSolid};

fn main() -> hagane::Result<()> {
    let policy = GeometryTolerance::new(1e-6, 1e-10, 0.0)?;
    let torus = NurbsTorusSolid::new(Frame3::IDENTITY, [16.0, 4.0], policy)?;
    let parts = torus.split_by_local_z(1.0, policy)?;
    parts.lower.validate(policy)?;
    parts.upper.validate(policy)?;
    let mesh = parts.lower.tessellate(0.1, policy)?;
    let step = parts.lower.export_step_mm(policy)?;
    println!("{} triangles, {} STEP bytes", mesh.triangles.len(), step.len());
    Ok(())
}
```

Each child retains exact restrictions of the original degree-(2,2) rational
NURBS surfaces. Knot insertion limits the minor parameter intervals without
renormalizing their original UV domains. Exact source parameter curves supply
shared major rims and minor edges; wall coedges retain affine original UVs.
An annular plane face closes the two cut circles, with an outer and inner wire
and rational pcurves obtained by affine projection of actual rim control points.
Matching weights/knots and measured control-point residuals bound the complete
cap boundary, in addition to same-parameter checks. Shared edges have exactly
two opposing oriented uses. Each child is a closed genus-one solid.

Literal height zero retains two whole minor quarters in each child. A resolved
nonzero cut retains two or four connected partial/whole minor strips depending
on its sign and selected side. Near-zero values do not snap to zero: unresolved
tiny strips refuse explicitly. The cut must lie strictly inside the tube height;
top/bottom clearance, annulus width and every retained partial strip must clear
the original full-body contact band and arithmetic reserves. Nonfinite heights,
contact, insufficient world precision and unrepresentable child volumes fail.
General oblique cuts and repeated child cuts remain unsupported.

For a horizontal slice, the cross-section area is
`4*pi*R*sqrt(r*r-z*z)`. Integrating gives each child's volume
`2*pi*R*r*r*(theta - sin(theta)*cos(theta))`, with the appropriate positive cap
angle. A positive odd-power series avoids cancellation for small angles;
checked binary scaling prevents avoidable intermediate overflow. The two
positive volumes conserve the source volume within floating-point arithmetic.
`bounds` returns the conservative positive-weight control hull of the actual
restricted patches, without claiming a tight analytic box.

Display meshes come from the actual restricted surfaces and plane cap. Shared
edge samples and vertices are reused exactly; annular cap triangles share both
actual cut rims. Equal physical-angle grids respect each original UV interval.
The whole wall-triangle ideal bound remains `(R+4*r)*h*h/8`, which also dominates
cap rim sagitta, with added restriction/frame/evaluation/cache arithmetic
reserves. These are binary64 engineering bounds, not interval certificates.
A common dyadic grid stops at 32 subdivisions per retained strip/major quarter,
with at most 33,024 triangles under the supported strip counts. Insufficient
chord error, arithmetic or resource budgets return explicit errors.

The existing bounded STEP writer exports each child's actual rational surfaces,
shared curves, plane cap and topology. This does not add generic STEP import.
Original code remains MIT OR Apache-2.0; no new dependency or external source
was introduced.

`NurbsTorusDemoSession::split_horizontal(height, chord)` serializes both actual
children completely before returning success. It leaves the accepted full torus
and all source point/segment/line/ray queries unchanged. The separate two-value
WASM transport takes local height and display chord error, with its own malformed
input latch. Standalone reports include the source placement and world cut plane.
Run `cargo run --example nurbs_torus_horizontal_split`; a ten-value JSON array
can supply the eight model parameters followed by cut height and child chord
error.

The browser can preview either child, download its actual STEP and restore the
source view. Source queries and the source STEP button still use the accepted
full torus; the preview identifies that distinction. Failed cuts or display
requests preserve the last accepted preview and source. See [torus demo setup](nurbs-torus.md).


## Mesh-free child point queries

`child.point_query(point, policy)` and `child.classify_point(point, policy)`
measure Euclidean distance to the retained circular meridian arc and the finite
annular cap. The cap's hole and plane extensions are not boundary surfaces.
The original full-source tolerance scale and world/frame arithmetic guards apply
before shortcuts. Resolved points return `Inside` or `Outside`; boundary witnesses
are evaluated on actual restricted wall faces in their original UV domains or on
the actual cap plane with Gram-coordinate UVs and oriented normals. A boundary
witness must have ten tolerance bands of physical cut-rim clearance. Ambiguous
trim-rim normals, unresolved thresholds and remote/nonfinite input return errors.

The session now caches the actual pair only after both child reports serialize
successfully. `query_horizontal_point(is_lower, point)` reads that pair without
rebuilding geometry, display or STEP. Rejected cuts, queries and source edits
preserve it; a successful source replacement invalidates it. The independent
four-value WASM transport accepts `[lower_flag, x, y, z]`, where exactly `1` means
lower and exactly `0` means upper. Queries before an accepted cut are rejected.
Run `cargo run --example nurbs_torus_horizontal_query`; optional fourteen-value
JSON input contains the eight source parameters, cut height, chord error, child
flag and world point. The browser's **Probe viewed cut part** uses the selected
accepted child; source point/segment/line/ray probes retain their source meaning.
Child line/ray queries are implemented below.


## Checked child mass moments

`child.mass_properties(policy)` returns uniform-density volume and world
centroid. `child.inertia_properties(policy)` additionally returns the full
symmetric inertia tensor about that centroid, expressed in world axes. The
actual restricted B-rep is validated before computing these metrics. No mesh
integration or source-body substitution is used.

Integrate the circular meridian disk segment about its local center. With
positive cap angle `theta`, its area is `A = r²(theta − sin(theta)cos(theta))`,
its signed first Z moment is `±2r³sin³(theta)/3`, and its raw second moments are
`Mx² = r⁴(theta/4 − sin(2theta)/6 + sin(4theta)/48)` and
`Mz² = r⁴(theta/4 − sin(4theta)/16)`. The sign is negative for a lower child.
Rotating this segment around the torus axis gives centroidal moments
`Ix = Iy = V(R²/2 + 3Mx²/(2A) + Mz²/A − zbar²)` and
`Iz = V(R² + 3Mx²/A)`. A rigid frame transforms the centroid and complete tensor.

For thin caps, the implementation evaluates normalized analytic binomial series
in `q² = sin²(theta/2)` instead of subtracting nearly equal raw Z moments.
The centered variance uses `4r²q⁴(E2 − E1²)`, where `Ek = Jk/J0` and
`Jk = integral[0,1] t^(2+2k) sqrt(1−q²t²) dt`. A bounded series tail and arithmetic
reserves guard positivity and conditioning. Checked binary products reject
unrepresentable moments without avoidable intermediate overflow. These checks
are binary64 engineering guards, not formal interval certificates.

The existing native split example and WASM split transport include optional
`mass_properties` reports for both actual children, with density, units and
centroid/world-axis conventions. Unavailable moments carry an explicit error;
optional metric failure does not turn a valid geometric split into failure.
The browser displays these child metrics separately from the full-source panel.

These formulas are independently derived from disk integration and rotational
symmetry; see the public calculus and parallel-axis references in
[references](references.md). No OCCT source or new dependency was used.


## Mesh-free child finite segments

`child.intersect_segment(start, end, policy)` returns ordered
`NurbsTorusSegmentHit` values and material intervals in the original segment's
`[0,1]` parameter. It reads the validated actual child; the display mesh is never
used. The existing certified source torus quartic solver supplies wall roots and
material intervals. Intersecting those intervals with the child's local-Z
halfspace adds the finite annular cut-cap crossing. Every retained wall or cap
hit is certified again through the actual child's point-query witness, preserving
its face ID, original surface UV, evaluated point, distance and oriented normal.

Endpoints, root separation and material intervals must resolve at the original
full-source tolerance scale. A forward-error check rejects shallow crossings.
Boundary endpoints, physical trim-rim hits, segments coincident with cap material
and unresolved contacts fail explicitly. Source quartic refusals remain
conservative even when a tangent or source-boundary endpoint lies in the
discarded halfspace; this API does not claim a general
trimmed-surface intersection solver. Accepted inside starts/ends clip material
intervals to zero/one; reversing a segment preserves geometry and reverses its
original parameterization. The cap hole and plane extensions carry no invented
surface hit.

`NurbsTorusDemoSession::intersect_horizontal_segment(is_lower, start, end)`
queries the cached actual split pair without rebuilding display or STEP. Failed
queries, cuts and source edits preserve the accepted pair; successful source
replacement invalidates it. The independent seven-value WASM transport uses
`hagane_nurbs_torus_horizontal_segment_begin/push/finish` and accepts
`[lower_flag, start_x, start_y, start_z, end_x, end_y, end_z]`; exactly one selects
lower and zero selects upper. Its malformed-input latch is independent of all
source, cut and point-query buffers.

Run `cargo run --example nurbs_torus_horizontal_segment` for a vertical cap/wall
intersection. Optional seventeen-value JSON input contains the eight source
parameters, cut height and chord error, child flag and six world coordinates.
The browser's **Probe viewed cut segment** reads the privately accepted selected
child. Full-source point, segment, line and ray probes keep their source meaning;
child probes preserve the rendered geometry, camera, prior point probe and STEP.
Repeated cuts and arbitrary oblique torus cuts remain unsupported; child lines
and rays are implemented below.


Native/WASM regression compares child-segment geometry, parameters, face IDs,
UVs and normals with the existing strict 32-EPS numeric checks. The measured
residual distance is checked independently against each report's actual hit and
surface-point coordinates. Its cross-platform bound propagates those coordinate
differences through the Euclidean norm, using the triangle inequality. A tiny
residual obtained by subtracting world coordinates does not inherit their
relative precision. This test does not round, replace or waive the kernel's
measured distance, its absolute physical limit or its forward-error guards.


## Mesh-free child lines and rays

`child.intersect_line(origin, direction, policy)` and
`child.intersect_ray(origin, direction, policy)` use the actual validated child
and preserve the original nonunit direction parameter in
`point = origin + direction * t`. Results reuse `NurbsTorusLineIntersection`:
ordered actual wall/cap witnesses and up to two material intervals, with at most
four hits. Ray intervals have `t >= 0`, and an origin inside this child clips its
containing interval to literal zero. Membership is that of the selected child,
not the original full torus. Lines can have a resolved boundary hit at `t = 0`;
ray origins in the boundary band explicitly fail.

The original anchor's full-source tolerance and world/frame arithmetic guards
run before cropping or a miss shortcut. An expanded full-source local enclosure
supplies outside endpoints; cropping at the child's cap would create boundary
endpoints and is deliberately avoided. The checked actual-child segment solver
supplies the intersections. Mapping back to the original parameter re-evaluates
the actual face/UV witness and checks reconstruction, measured residual,
absolute forward error, ordered intervals and physical widths/gaps. Nonfinite
or zero directions, unrepresentable mappings, physical cut rims, contacts and
unresolved shallow crossings return errors. Conservative source contacts remain
unsupported even in the discarded halfspace or behind a ray's origin.

`NurbsTorusDemoSession::intersect_horizontal_line(is_lower, origin, direction)`
and `intersect_horizontal_ray` query the accepted actual pair read-only. Separate
seven-value `hagane_nurbs_torus_horizontal_line_begin/push/finish` and
`hagane_nurbs_torus_horizontal_ray_begin/push/finish` buffers take
`[lower_flag, origin_x, origin_y, origin_z, direction_x, direction_y, direction_z]`.
Exactly one selects lower and zero selects upper. Each buffer has its own finite
input validation and malformed-input latch. Rejected queries, cuts and model
edits retain the pair; successful source replacement invalidates it.

Run `cargo run --example nurbs_torus_horizontal_line` or
`cargo run --example nurbs_torus_horizontal_ray`. Optional seventeen-value JSON
input contains eight source parameters, cut height/chord error, child flag,
world origin and direction. The browser's viewed-child line/ray probes use its
private accepted selection and preserve camera, mesh, STEP, prior point/segment
probes and the other line/ray result. Separate full-source probes keep their
source meaning. Native/WASM geometry comparisons and independently checked,
coordinate-propagated measured-distance comparisons use the same policy described
for child segments above. Repeated child cuts, arbitrary oblique torus cuts,
horizontal-child STEP import and general torus Booleans remain future work.
