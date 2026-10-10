# Mesh-free point queries on oblique frustum children

`NurbsObliqueFrustumSolid::classify_point(point, policy)` classifies a world
point as `Inside`, `Outside` or `Boundary` against the actual finite child.
It uses the validated source frustum and the retained planar cut, with no mesh
sampling. `Boundary` means Euclidean distance within the caller's length band.

The source's finite side, caps and circular rims supply the original boundary
distance query. A normalized oriented cut-plane distance supplies the retained
halfspace. A resolved outside result for either constraint establishes outside
the child. Source-boundary results require enough cut clearance to retain the
source boundary witness. Cut-boundary results require an admissible orthogonal
plane-foot witness inside the source. Both resolved inside constraints establish
an inside result.

At the new elliptical rim, combining two individual boundary bands does not
give Euclidean distance to the child. For example, two orthogonal offsets of
0.8 tolerance have diagonal distance greater than tolerance. Ambiguous rim
queries therefore return an explicit unsupported error instead of a false
boundary result. This conservative domain also refuses unresolved threshold
comparisons, distant-coordinate precision and invalid inputs. It does not claim
an unrestricted closest-point solver for elliptical trims.

## Native and browser use

Run `cargo run --locked --example nurbs_frustum_oblique_classify`. Optional JSON
input supplies the [plane-split demo's sixteen values](nurbs-frustum-plane-split.md)
followed by world query XYZ. Successful reports retain both actual closed
children and add `point_query` with `point`, lower/upper `classes` and
`mesh_free:true`. Query errors do not masquerade as successful classifications.

The existing `/nurbs-frustum-plane-split.html` demo has a separate world-point
probe. It uses the last accepted model even while model fields are being edited.
A failed query preserves its last result, accepted solid, camera and STEP
download. Shape building remains available independently of query admission.

The new WASM begin/push/finish interface accepts exactly nineteen finite values;
the existing sixteen-value shape interface remains available. Native and WASM
share the same Rust classifier.

Independent tests cover diagonal rim offsets, retained and removed source
boundaries, actual sides and cut caps, normal reversal, rigid placement, scaled
models, relative tolerance and invalid/far queries. Native/WASM report parity
and browser tests verify successful probes and rejection recovery against the
accepted model, including preserved camera, rendered child and STEP export.

This implementation uses Euclidean distance, orthogonal projection and convex
halfspace intersection from public mathematics. No OCCT source or new dependency
is used; original code is MIT OR Apache-2.0. See [references](references.md).
