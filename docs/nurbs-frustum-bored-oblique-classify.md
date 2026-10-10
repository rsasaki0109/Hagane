# Mesh-free point classification of bored oblique children

`NurbsBoredObliqueFrustumSolid::classify_point(point, policy)` classifies the
actual retained finite solid as `Inside`, `Outside` or `Boundary`. It does not
use the display mesh or substitute the filled outer source. Ambiguous inputs
return explicit errors; a failed query never changes geometry or history.

The original bored solid's finite outer wall, inner wall and annular cap
classifier executes first, including its world/frame precision checks.
The relative length band uses the complete original frustum extent, rather
than query distance, bore radius or the smaller child. The actual retained cut
cap then supplies a normalized plane distance with its own arithmetic reserve.

Being outside the original material or farther than the band outside the
retained halfspace proves `Outside`. Being strictly inside original material
and beyond the plane band proves `Inside`. An original boundary witness is
retained only when cut-plane clearance exceeds ten full source bands.

Near the cut plane, the orthogonal foot must be strictly inside the actual
original annular material to certify `Boundary`. This proves membership in the
finite annular cut cap; a foot in the cylindrical void or beyond an ellipse
cannot manufacture a filled cap. Close outer/inner elliptical rims and
unresolved threshold comparisons remain unsupported. Separate wall and plane
bands are never substituted for finite-corner distance.

The workflow's existing read-only `query` command now dispatches to this actual
child type. It returns the same small point report without replaying the graph,
generating a mesh, replacing shapes or modifying Undo/Redo. The browser probe
uses the accepted selected component; rejection preserves the last successful
query, camera, geometry and STEP output.

After rebuilding the bored-oblique example, this command probes lower-child
material in world millimetres:

```json
{"command":"query","component":0,"point":[6,0,6]}
```

Tests cover holes, retained and removed halfspaces, original annular caps,
finite cut-cap witnesses, diagonal rim ambiguity, rotations, scale changes,
relative policy, nonfinite input and coordinate precision. Native, WASM and
browser probes share the Rust kernel implementation. Line/ray queries now use
these actual finite witnesses.
Skew planes remain unsupported in the parallel-cut API; the separate interior-cut API checks ordered nonparallel boundaries.

Original code is MIT OR Apache-2.0. This uses finite annular boundary distances,
halfspace distance lower bounds and an orthogonal finite-face witness. No OCCT
source or new dependency is used.

[Repeated exact-basis parallel cuts](nurbs-frustum-parallel-slabs.md) now
produce separate typed slab children. Skew planes remain unsupported in the parallel-cut API; slab
metrics and queries have separate explicit unsupported status.
