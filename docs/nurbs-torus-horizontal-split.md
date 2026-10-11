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
General oblique cuts, repeated child cuts and child classification/intersection
or mass-property APIs remain unsupported.

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
