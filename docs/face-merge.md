# Coplanar face merging

![Actual WASM contact solid after face merging](face-merge.png)

`merge_coplanar_faces(&solid, GeometryTolerance)` joins edge-connected planar
straight-edge faces on certified identical supports. It returns a newly validated
`Solid`; the operand is unchanged. It removes internal face boundaries and rebuilds
outer/hole wires, shared topology and pcurves. Display triangles are generated
only after the resulting analytic B-rep is validated.

## Supported domain

The input must be a geometrically valid, closed planar straight-edge solid, with
at most 512 faces and 4096 coedges. Plane identity is certified across different
origins and UV frames by exact scalar triple products of the supplied binary64
vectors: both target axes must lie in the source plane, and the exact origin
difference must lie in that plane. Outward normals must have equal orientation.
The public `same_plane_support(&first, &second)` checks checked plane supports
independently of outward orientation.

This includes different in-plane basis rotations/reflections and shifted origins,
including tilted planes. Independently rounded representations that define
slightly different binary64 planes remain separate even if their nominal design
plane was the same. There is no tolerance-based snapping or healing.

Only adjacency through a shared edge joins a component. Edge-disconnected regions
on the same plane remain separate faces. Curved input and invalid topology fail
explicitly. Pinched or ambiguously branching component boundaries are unsupported.
Structural validation is not a general geometric self-intersection detector.

## Boundary construction and invariants

Group adjacent faces with certified equal supports. Remove coedges whose two
incident faces belong to the same group. Traverse the remaining directed boundary
graph into closed loops, using the representative face's UV orientation. Positive
UV area identifies outer rings; negative rings must belong to exactly one outer
ring. This supports concave boundaries and planar holes without filling the holes.

Strict planar sewing reconstructs the solid. Every output vertex and edge must
correspond to an original boundary vertex/edge. Affine pcurves on certified equal supports are transformed into the representative
frame using local basis coefficients and origin offset, then reversed as needed
for the output edge parameter. Exact axis permutations/sign changes use exact
coefficients. This preserves original UV data instead of reprojecting every
world-space boundary vertex. Final geometric checks reject unresolved conversions.

The result must retain exact bounds, conserve analytic volume within relative
`1e-10`, and not increase the face count. Final validation checks trims, plane
geometry, pcurves, signed edge uses, vertex links and closed connectivity. Internal
vertices/edges disappear; exterior collinear subdivisions remain. Collinear edge
simplification, curved-surface merging and tolerant healing are future work. Applying this operation again preserves topology counts.

## Usage

```rust
use hagane::*;
fn main() -> Result<()> {
    let a = BoxSpec { min: Point3::new(0., 0., 0.), size: Vec3::new(4., 4., 4.) };
    let b = BoxSpec { min: Point3::new(4., 1., 1.), size: Vec3::new(2., 2., 2.) };
    let policy = GeometryTolerance::default();
    if let BoxBooleanResult::Solid(part) =
        boolean_boxes(a, b, BoxBooleanOperation::Union, policy)? {
        let merged = merge_coplanar_faces(&part, policy)?;
        assert_eq!(merged.shell.faces.len(), 11);
        // The remaining contact-plane face has an inner boundary.
    }
    Ok(())
}
```

```sh
cargo run --locked --example face_merge -- -2
cargo run --locked --example part -- 18
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Select **Merge coplanar faces**. This applies merging to the previous **Fuse
contacting boxes** B-rep. Its exact volume stays **122880 mm³**; faces decrease
from **26 to 10** and edges from **52 to 34**. The slider changes attachment
position, and the same Rust operation runs natively and in WASM.

Native tests cover a full-face union reduced to six faces, a partial-face union
with a planar hole, rectangular through-hole caps, concavity and disconnected
coplanar regions, rigid placement with preserved UV, small dimensions, nearly
parallel feature preservation, invalid/curved inputs and idempotence. Tests compare
bounds, volume and material/boundary classification and check outward triangles
and closed mesh seams. WASM verifies native geometry parity at four offsets and
error recovery. Browser tests exercise all 19 presets and capture the actual demo.


A second [tilted-frame fixture](reframed-merge.md) exercises independently
parameterized faces, exact 3D predicates and native/WASM/browser parity.
