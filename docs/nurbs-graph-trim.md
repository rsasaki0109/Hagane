# Exact rectangular restriction of a closed NURBS graph solid

`NurbsGraphSolid::trimmed_uv` retains a rectangle inside the solid's current
source U/V domain. It restricts the roof by exact knot insertion and constructs
a matching base and four ruled NURBS walls. The new walls' upper boundaries
are exact quadratic curves on the roof. The result retains eight shared
vertices, twelve shared edges and six oriented faces with surface pcurves.
This is a scoped solid restriction, not a general Boolean algorithm.

```rust
use hagane::{NurbsGraphSolid, Tolerance};

let tol = Tolerance::default();
let source = NurbsGraphSolid::new([80.0, 60.0, 20.0], 30.0, tol)?;
let part = source.trimmed_uv([[0.2, 0.8], [0.25, 0.9]], tol)?;
part.validate(tol)?;
let display = part.tessellate_bounded(0.2, 65_536, tol)?;
# Ok::<(), hagane::Error>(())
```

The domain stays in the original source parameters. Nested restrictions must
remain inside the currently retained rectangle; they cannot restore removed
material. Finite ordered ranges, resolvable physical widths and parameter
conditioning are checked. Zero-width, inverted, outside or unresolved windows
return explicit errors. Existing rigid placement is preserved.

For retained ranges `[a,c]` and `[d,e]`, exact volume is

```text
du = c-a, dv = e-d
I(a,c) = du * (m*(1-m) - du*du/12), where m=(a+c)/2
volume = L*W * (H*du*dv + 4*b*I(a,c)*I(d,e))
```

The full-domain formula reduces to `L*W*(H+b/9)`. Rectangular partitions
conserve this analytic volume. Trimmed or placed bounds are conservative
control-hull enclosures; only the full identity graph reports exact extrema.

Tessellation evaluates every retained face in its actual parameter domain,
with a common dyadic grid and shared topological nodes. Hessian bounds cover
both the roof and the newly curved ruled walls. Triangle bounds include the
world arithmetic allowance. No mesh clipping or mesh Boolean constructs the
solid; the mesh is generated afterward from its retained B-rep.

The browser `graph-solid.html` edits source limits together with dimensions,
roof control offset, rotation, translation and chord error. The native JSON
example is `cargo run --example nurbs_graph_trimmed`. General nonrectangular
solid trims, arbitrary rational sewing, curved Booleans and NURBS STEP remain
unsupported.
