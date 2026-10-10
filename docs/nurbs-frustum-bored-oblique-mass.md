# Analytic mass of bored oblique frustum children

`NurbsBoredObliqueFrustumSolid::mass_properties(policy)` provides the actual
retained child's analytic volume and world centroid. `inertia_properties(policy)`
adds its uniform-density centroidal tensor in world axes. Length units follow
the model; the millimetre demo reports volume in mm³ and inertia in mm⁵ at
density one. No display mesh is used for these metrics.

The independently checked outer and removed cylinder child moments are
converted to one source-local normalized scale. Their signed first moments give
the retained centroid. Signed parallel-axis composition about that centroid
gives its covariance; the trace relation gives inertia, and the actual rigid
frame rotates the tensor with R I Rᵀ. Large world translations are applied
only after the local moments have been combined.

Existing analytic child moments carry centroid, covariance and volume error
budgets into this subtraction. Cancellation, scale conversion, positive
covariance and physical centroid precision are guarded. A spectral error
reserve is subtracted before Cholesky checking; unresolved values are never
clamped into a valid tensor. Exponent-managed products restore physical units
without introducing premature overflow or underflow.

Very thin material, nearly uniform but unequal tapered radii, extreme scales
or insufficient precision can still return explicit `Unsupported` metrics.
The workflow then retains the actual validated geometry, volume, bounded
mesh and exact STEP, while reporting `mass_properties.ok = false`. No source
centroid, tensor or sampled approximation is substituted. Other failures
still reject the output transaction.

Independent tests use polynomial moments of a tilted annular cylinder and
parallel-axis conservation of both actual children against the bored parent.
They cover contracting/expanding taper, rotations, translations, scale changes
and unavailable-metric retention. Native, WASM and browser paths use the same
kernel metrics, and browser history edits recompute the actual selected body.

Original code is MIT OR Apache-2.0. This uses the existing independent analytic
cone/cylinder integrals, covariance identity and parallel-axis theorem. No OCCT
source or new dependency is used. [Mesh-free point queries](nurbs-frustum-bored-oblique-classify.md) are supported;
segment/line/ray queries and repeated cuts remain unsupported.
