# Analytic mass properties of oblique frustum children

The closed children returned by [plane splitting](nurbs-frustum-plane-split.md)
provide `mass_properties(policy)` and `inertia_properties(policy)`. Results
contain volume, the world centroid and, for inertia, the complete symmetric
centroidal tensor expressed in world axes. Density is one; with millimetre
coordinates, volume is mm³ and inertia is mm⁵. Multiply by physical density
to obtain mass and mass inertia in consistent units.

```rust
// split is the validated NurbsFrustumPlaneSplit from the plane-split example.
let lower = split.lower.inertia_properties(policy)?;
let upper = split.upper.inertia_properties(policy)?;
assert!((lower.volume + upper.volume - stock.volume(policy)?).abs() < 1e-8);
```

## Mathematics and conditioning

Calculations use normalized source-local coordinates before transforming the
centroid and rotating the tensor as `R I Rᵀ`. They do not subtract raw moments
about a potentially distant world origin. Exact equal radii use polynomial
disk-column integrals. A nonzero taper uses signed differences of supporting
cones, with the actual cut ellipse's area centroid and covariance, rather than
the cut-cap parameter origin or the caller's plane origin.

For a cone with apex A, base centroid B, base covariance C and volume V, write
d = B − A. Its first moment is `V*(A + 3*d/4)` and raw second moment is
`V*(A*Aᵀ + 3*(A*dᵀ+d*Aᵀ)/4 + 3*(d*dᵀ+C)/5)`.
Subtracting the appropriate original-cap cone yields each child's moments.
The upper child is evaluated directly rather than subtracting its moments from
the full source. Volume uses the existing cancellation-avoiding split formula.

Absolute operand budgets detect cancellation in cone differences and reflected
cylinder moments. Centroid and covariance conditioning checks reserve linear
tolerance; a shifted Cholesky check rejects unresolved positive definiteness.
These are conservative engineering guards, not formal interval certificates.
`mass_properties` also applies covariance conditioning, so a centroid-only
request can conservatively reject when the full moment calculation is unresolved.
Overflow, underflow, distant placement, very thin pieces and nearly uniform but
nonzero tapers can return `Error::Unsupported`. No radius snapping, sampled
quadrature fallback or eigenvalue clamping conceals that failure.

## Reports and verification

The existing native/WASM/browser plane-split demo reports each child's
`mass_properties.ok`. Success includes its actual volume, centroid and full
inertia with units and centroid reference. An unsupported metric instead has
`ok:false` and an error, while successful B-rep, mesh, volume and STEP remain
available. The browser displays centroid and diagonal moments and names metric
failure explicitly.

Independent tests integrate positive cylindrical-coordinate moments for
contracting, expanding and equal-radius sources. They check all tensor entries,
rigid placement, plane reversal, parallel-axis conservation, scales 1e±50 and
explicit thin/near-uniform/overflow rejection. WASM and browser checks add exact
cylinder disk-moment oracles and complete native report parity.

Cone moments follow integration of homothetic planar sections; disk moments,
covariance and the parallel-axis theorem are public mathematics. This independent
implementation is MIT OR Apache-2.0, uses no OCCT source and adds no dependencies.
See [mathematical and dependency references](references.md).
