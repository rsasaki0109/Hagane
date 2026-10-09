# Rigid placement of polynomial NURBS graph solids

`NurbsGraphSolid::transformed` applies a checked right-handed rigid transform
to the retained vertices, rational curves and surfaces. Edge references,
coedge orientation and surface pcurves remain attached to the same geometry.
Scale, shear and reflection are excluded by the existing `Transform` contract.

The wrapper stores a cumulative placement. It regenerates the expected B-rep
from the original dimensions and roof control offset under that placement,
so a public geometry or topology mutation is rejected by validation, volume,
bounds and display. Repeated transforms compose in world coordinates; they
do not repeatedly round the existing source control nets.

```rust
use hagane::{NurbsGraphSolid, Transform, Vec3, Tolerance};

let tol = Tolerance::default();
let local = NurbsGraphSolid::new([80.0, 60.0, 20.0], 30.0, tol)?;
let rotate = Transform::rotation(Vec3::new(0.0, 1.0, 0.0), 0.4)?;
let place = Transform::translation(Vec3::new(100.0, -20.0, 10.0))?
    .compose(rotate)?;
let placed = local.transformed(place, tol)?;
let display = placed.tessellate_bounded(0.2, 65_536, tol)?;
# Ok::<(), hagane::Error>(())
```

Exact analytic volume is invariant under placement. Identity placement retains
the original exact graph bounds. A nonidentity placement returns a conservative
world-space control-hull enclosure, not exact rotated rational extrema.
Display evaluates the placed B-rep directly, shares topological node identities
and keeps separate face normals. The approximation bound includes a world
coordinate arithmetic allowance; unresolved or overflowing requests fail
explicitly. It is an engineering guard, not formal interval arithmetic.

The `graph-solid.html` demo edits rotation about world Y and translation in
addition to dimensions, roof control offset and display error. Native and WASM
use the same placed geometry generator; viewer centering changes only rendering.
Original world coordinates remain in the generated data.

This adds rigid placement only to the scoped graph-solid API. General NURBS
solid validation, sewing, curved Booleans and NURBS STEP interchange remain
unsupported.
