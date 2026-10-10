# Oblique cuts through a retained frustum bore

`NurbsBoredFrustumSolid::split_by_plane(&plane, policy)` returns
`NurbsBoredFrustumPlaneSplit { lower, upper }`. Each child is an actual closed
`NurbsBoredObliqueFrustumSolid`, with sixteen vertices, twenty-four shared edges,
ten oriented faces and two annular cap wires. The lower child retains the
original lower cap; ordering is independent of the supplied plane normal sign.

```rust
use hagane::*;
# fn example() -> Result<()> {
let policy = GeometryTolerance::new(1e-6, 1e-10, 0.)?;
let stock = NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy)?;
let bored = stock.bore_coaxial(4., policy)?;
let normal = Vec3::new(0.2, 0., 1.).normalized()?;
let u = Vec3::new(0., 1., 0.);
let plane = Surface::Plane {
    origin: Point3::new(0., 0., 12.), u, v: normal.cross(u),
};
let parts = bored.split_by_plane(&plane, policy)?;
parts.lower.validate(policy)?;
let mesh = parts.lower.tessellate(0.1, policy)?;
let step = parts.lower.export_step_mm(policy)?;
assert!(!mesh.triangles.is_empty());
assert!(step.contains("CLOSED_SHELL"));
# Ok(()) }
```

The supported plane forms a resolved complete closed section strictly between
both original caps. Cap crossings, tangencies, nearly parallel unresolved cuts,
nonfinite inputs and insufficient coordinate precision reject explicitly.
The original full-source tolerance and positive bore-wall clearance are checked
before restricting the independently certified outer frustum and inner cylinder.
No partial split is returned.

The new cut cap's inner pcurves are affine projections of the actual inner rim
onto the actual outer cap surface. Their positive rational bases retain the
spatial curve's weights, degree and knots. A bound on every control-point
residual proves correspondence along the entire rim, even though the two cut
caps have different local origins. Inward walls and inner wires reverse the
appropriate oriented uses; every retained edge has two opposed incidences.
The privately retained complete construction certificate checks the actual
B-rep, rather than substituting filled outer pieces or mesh Boolean results.

Volumes use checked analytic outer and inner child volumes. Subtraction and
parent conservation have explicit floating-point conditioning guards;
unresolved very thin material can be unsupported even when the parent exists.
[Analytic centroids and inertia](nurbs-frustum-bored-oblique-mass.md) now have
checked availability. Unresolved metrics preserve geometry and export.
[Mesh-free point classification](nurbs-frustum-bored-oblique-classify.md) now
uses finite annular witnesses. [Finite segment queries](nurbs-frustum-bored-oblique-segment.md)
retain both material intervals and actual face UVs. [Line/ray queries](nurbs-frustum-retained-line.md) preserve original parameters.
Geometry, volume, bounded display and exact STEP export remain usable. Repeated cuts, general curved Booleans and generic
bored-frustum STEP import are also unsupported.

Display approximates all eight actual rational walls with one topology-keyed
dyadic grid. Rational endpoint weights map each shared generator to physical
line travel. Restricted patch bounds include boundary replacement errors.
Annular cap strips use both actual rim loops; their bore-side intrusion is
checked against the certified inner-rim chord bound. The display tolerance is
an approximation budget, while exported geometry remains the actual B-rep.

The workflow `plane_split` node now accepts one explicitly selected canonical
or bored component. [This example](nurbs-frustum-workflow-bored-oblique-example.json)
connects stock → through bore → two slanted annular parts. The browser preset
exposes the hole radius and world plane, retains unchanged graph prefixes and
supports Undo/Redo and per-child STEP export. Further cuts of the resulting
bored-oblique components fail explicitly.

Original code is MIT OR Apache-2.0. This composes Hagane's independently
implemented rational plane sections and finite ruled restrictions, together
with affine rational-curve projection and positive control-net bounds. No OCCT
source or new dependency is used.
