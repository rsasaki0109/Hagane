# Provenance, references, and licenses

The kernel geometry, B-rep construction/validation, restricted difference,
analytic metrics, WASM interface, WebGL viewer, and tests were written
independently for Hagane. No OCCT source was read, copied, translated, linked,
or vendored. OCCT is a functionality reference only; this project does not
claim compatibility with its data structures or APIs.

## Mathematical and specification references

- Boundary representation, manifold topology, and Euler characteristic:
  [Boundary representation](https://en.wikipedia.org/wiki/Boundary_representation),
  [Euler characteristic](https://en.wikipedia.org/wiki/Euler_characteristic).
- Analytic surface parametrization and oriented normals: standard Cartesian
  plane/cylinder equations; [Parametric surface](https://en.wikipedia.org/wiki/Parametric_surface).
- Oblique circular-wall sections: substitution into a Cartesian plane yields
  a sinusoidal height graph and an affine unit-circle image (ellipse).
  Harmonic wall flux uses elementary trigonometric integrals. Linear
  interpolation error uses the second-derivative bound; shared edge sampling
  keeps adjacent analytic trims conforming. All are independently implemented
  from mathematical formulas, with no added dependency or copied source.
- Full-ellipse planar trims: inverse affine unit-circle coordinates and
  Cartesian line/circle roots; Green's theorem integrates ellipse arc area.
  Chord interpolation error follows Taylor's second-derivative bound, and
  boundary-distance bounds follow the Hausdorff-distance triangle inequality.
  Shared ellipse/pcurve parameters and coefficient-error sums preserve analytic
  B-rep provenance. Independently implemented formulas; no reference source code
  or new dependencies. See [ellipse planar trims](ellipse-planar.md).
- Skew circular face subdivision: orthonormal basis change and circular angular
  addition preserve the physical translation while rebasing child parameters.
  Rim/coedge refinement uses shared B-rep traversal and affine parameter
  substitution; implemented independently without new dependencies.
- Rigid coordinate frames and rotation: orthonormal basis projection and
  [Rodrigues' rotation formula](https://en.wikipedia.org/wiki/Rodrigues%27_rotation_formula).
  Implemented from the mathematical formulas, without reference source code.
- Circular arc area integrals: [Green's theorem](https://en.wikipedia.org/wiki/Green%27s_theorem).
  Line/circle and circle/circle distance candidates follow Cartesian equations
  and their stationary conditions. All formulas are implemented independently;
  no reference code or prose is incorporated.
- Analytic plane/plane solve: normal cross product and local linear constraints,
  [intersection of two planes](https://en.wikipedia.org/wiki/Plane_(geometry)#Intersection_of_two_planes).
  Line/cylinder roots use radial closest approach and the Cartesian circle
  equation; axial generators use a linear height interval. Implemented
  independently from mathematical formulas, without reference source code.
- Planar face clipping: analytic line/line and line/circle roots, ordered
  boundary events, midpoint region classification and sorted interval
  intersection. Surface pcurves use shared parameters; finite line segments
  reparameterize both UV curves to [0,1]. Implemented independently from these
  standard equations and interval operations, without new dependencies.
- Planar face subdivision: directed boundary paths, opposite uses of a shared
  chord, containment-based hole ownership and affine parameter substitution.
  Implemented independently as B-rep topology operations; no source code or
  additional library is incorporated.
- Bounded rim/wall refinement: angular parameter translation, rotation of the
  local circle basis and subdivision of a rectangular cylinder domain along
  an axial generator. Opposite rim edges and cap pcurves share the subdivision
  angle. Hole ownership uses analytic line/arc classification. Implemented
  independently from these formulas/topology operations, with no new dependency.
- Volume integration: [Divergence theorem](https://en.wikipedia.org/wiki/Divergence_theorem).
- Skew circular boundary distance: orthogonal plane/segment projection, convex
  parallelogram coordinates, circle sagitta and the distance-function bound from
  [Hausdorff distance](https://en.wikipedia.org/wiki/Hausdorff_distance).
  Independent chord-patch refinement uses analytic generators and checked
  arithmetic guards, without adding a dependency or using a display mesh.
- Chordal approximation bound: [Sagitta](https://en.wikipedia.org/wiki/Sagitta_(geometry)).
- Exact predicate background: Jonathan Richard Shewchuk, *Adaptive Precision
  Floating-Point Arithmetic and Fast Robust Geometric Predicates* (1997),
  [publication](https://www.cs.cmu.edu/~quake/robust.html);
  [IEEE 754 binary64 representation](https://en.wikipedia.org/wiki/Double-precision_floating-point_format).
  Hagane uses a conservative floating filter and an independently designed
  dyadic-integer fallback, not Shewchuk's expansion routines. No reference code
  or prose was copied, translated, or vendored; no predicate dependency is linked.
- Simple polygon validation and containment: standard orientation determinants,
  Euclidean point/segment distance, and [ray casting point-in-polygon](https://en.wikipedia.org/wiki/Point_in_polygon#Ray_casting_algorithm).
  Mixed line/arc classification splits arcs at Y extrema and solves analytic
  horizontal ray/circle roots with half-open crossings. Cap conformity restores
  boundary vertices removed by display triangulation, using exact collinearity.
  Implemented directly in Rust, without importing predicate source code.
  Polygon-prism circular bore containment combines exact center classification
  with Euclidean distance to every boundary segment: a connected disk cannot
  leave a simple polygon or enter a polygon opening without crossing a boundary
  (see the
  [Jordan curve theorem](https://en.wikipedia.org/wiki/Jordan_curve_theorem)).
  Skew-stock tool containment follows a moving-coordinate change: the fixed
  world-axis center traces a segment in the translated profile over the actual
  tool depth and selected entry cap (the full height for through cuts). Endpoint
  classification plus segment/boundary separation certifies the complete swept
  disk. Dividing the gap by the wall-slope norm gives a conservative physical
  clearance bound by Cauchy–Schwarz. Opposing blind-tool separation uses
  disjoint axial intervals or disjoint XY disks: either positive gap certifies
  separation of the Cartesian-product cylinders. Implemented independently from these
  mathematical facts, with no copied code and no additional dependencies.
  Exact removed volumes use circular area times depth; display volume bounds
  use the inscribed circle chord/sagitta bound. No reference code or prose was copied.
- Rational B-spline formulas and homogeneous evaluation: Les Piegl and Wayne
  Tiller, *The NURBS Book*, second edition (1997); Carl de Boor, *A Practical
  Guide to Splines*. Mathematical background also appears in
  [de Boor's algorithm](https://en.wikipedia.org/wiki/De_Boor%27s_algorithm) and
  [B-spline derivative expressions](https://en.wikipedia.org/wiki/B-spline#Derivative_expressions).
  Hagane implements these formulas independently; no book/page source code or
  prose was incorporated, and no additional NURBS library is linked.
- Rational curve refinement uses homogeneous knot insertion and
  [de Casteljau subdivision](https://en.wikipedia.org/wiki/De_Casteljau%27s_algorithm),
  independently implemented from the published mathematical formulas. Positive
  rational Bernstein weights give a convex combination of Euclidean controls;
  convexity of distance to a chord segment supplies the adaptive display bound.
  The numerical allowance and resource policy are Hagane implementation choices,
  not claims supplied by those references. No reference code or prose was copied
  and no new dependency is required.
- Tensor-product surface knot insertion applies the same homogeneous formula
  independently along one axis while retaining a common control-net weight
  scale. Isoparametric curves follow by contracting the fixed-axis B-spline
  basis; the remaining coefficients and degree/knots form a rational curve.
  Rectangular boundaries use these curves with same-parameter affine UV maps
  and counterclockwise UV traversal. These are direct consequences of the
  tensor-product definition, implemented independently from the references
  above without copied code or additional dependencies.
- Browser integration specifications: [WebAssembly core specification](https://webassembly.github.io/spec/core/),
  [WebGL 1.0 specification](https://registry.khronos.org/webgl/specs/latest/1.0/).
- Planar polygon triangulation: [Mapbox Earcut](https://github.com/mapbox/earcut),
  via the independent Rust package below, used only for the display mesh.

These links document mathematical/specification background. No prose or source
from the reference pages is incorporated. The formulas and design decisions
used here are recorded in [design.md](design.md).

## Rust dependencies (Cargo.lock)

| Package | Version | Role | License | Source |
| --- | --- | --- | --- | --- |
| earcutr | 0.4.3 | Planar display polygon triangulation | ISC | https://github.com/frewsxcv/earcutr |
| itertools | 0.11.0 | earcutr dependency | MIT OR Apache-2.0 | https://github.com/rust-itertools/itertools |
| either | 1.19.0 | itertools dependency | MIT OR Apache-2.0 | https://github.com/rayon-rs/either |
| num-traits | 0.2.19 | earcutr dependency | MIT OR Apache-2.0 | https://github.com/rust-num/num-traits |
| autocfg | 1.5.1 | num-traits build dependency | MIT OR Apache-2.0 | https://github.com/cuviper/autocfg |

All are Rust packages. Their license declarations and actual bundled license
texts were checked in the downloaded, checksum-verified Cargo artifacts.
Copies of notices are in [third-party](third-party/). ISC and MIT terms require
retaining their notices when redistributing incorporated dependency code;
Apache-2.0 is an alternative for the dual-licensed packages. Original Hagane
code uses [MIT](../LICENSE-MIT) OR [Apache-2.0](../LICENSE-APACHE).

## Optional development tools

`package-lock.json` pins [Playwright](https://github.com/microsoft/playwright)
1.62.1 and playwright-core (Apache-2.0); platform-conditional `fsevents` (MIT) is
macOS-only. None is loaded by the browser application or used in the Rust
geometry kernel. Playwright's bundled Apache license/notice is retained in
`docs/third-party`. Node, Python, Rust/rustup, Chromium and FFmpeg are external
development tools, not embedded kernel dependencies. The included GIF was
captured from the actual app with Chromium/Playwright and encoded with FFmpeg.

### Planar cut graphs

The multi-interval face subdivision uses elementary oriented planar graph cycle
tracing: retain directed boundary subedges in each half-plane, add oppositely
oriented exact interval chords, then trace cycles through their shared vertices.
Green's theorem gives line/arc signed wire area; analytic point-in-ring tests
assign negative cycles to positive cycles. This independently implemented
construction reuses the existing documented clipping and arc-refinement math.
It adds no dependencies and uses no OCCT source.

Repeated crossings retain original curve parameters and refine in descending
parameter order. This elementary interval-subdivision rule keeps unprocessed
parameters on the first surviving subedge; normalized straight parameters are
rescaled to that subinterval, while arc parameters remain angular. Rim/frame
refinement follows the existing documented rotation formulas. The independent
implementation introduces no library or OCCT dependency.

### Periodic seam refinement

Circle periodicity allows a change of seam angle without changing its geometry.
Exact quarter arcs use the existing circular parametrization and orthonormal
frame rotation, with corresponding planar angular pcurves. Cylinder generator
seams and rectangle trims share these new rim vertices. Seam choice maximizes
angular clearance over a finite candidate set; chord clearance is checked via
`2*r*sin(delta/2)`. This independent implementation introduces no dependency and
uses no OCCT source. Exact geometry remains separate from display tessellation.

### Exact planar sewing

Independent planar patches are assembled as an oriented combinatorial
boundary representation. Exact coincident points define shared vertices;
unordered endpoint pairs define shared straight edges; per-face affine UV
maps use the same normalized edge parameter. Collinearity is tested with the
existing filtered exact orientation predicate in all three coordinate
projections. Subdivision points are ordered by a dominant coordinate and
propagated to all incident uses. Existing edge-use, vertex-link, connectivity,
trim and positive-volume validation establishes the supported shell invariants.
No snapping/healing is performed. This independently implemented construction
uses no new dependency and no OCCT source.

### Solid point classification

Ray crossing parity and oriented entry/exit counts classify points inside a
closed oriented polygonal boundary. Analytic line/plane intersections and the
existing exact 2D polygon membership predicate test each trimmed face, without
mesh triangulation. Euclidean point/trim distance defines a boundary band;
ambiguous vertex/edge/near-parallel rays are retried, and two independent rays
must agree. Metric 3D calculations remain checked f64 operations. This is an
independent implementation of elementary ray casting, with no new dependency
and no OCCT source.

### Solid plane partition

A plane defines two signed half-spaces. Original straight edges crossing it
receive one canonical shared intersection vertex. Existing analytic planar
clipping yields material intervals; oriented boundary graphs trace retained
face regions and section loops, including polygon holes. Section edges combine
face orientation with half-space orientation to construct complementary caps.
The existing exact planar sewing machinery assembles both parts, and analytic
volume conservation checks the result. This independent construction uses no
mesh modeling, no new dependency and no OCCT source.

### Convex solid intersection

A closed convex polyhedron is the intersection of its outward supporting
half-spaces. Sequential clipping by the second operand's half-spaces constructs
their common material. Plane cuts reuse exact edge intersections, cap graphs
and sewing; convexity is checked before using the half-space representation.
Background: S. Boyd and L. Vandenberghe, *Convex Optimization*, Cambridge
University Press, 2004, §2.2 (hyperplanes, half-spaces and polyhedra), available
from [Stanford](https://web.stanford.edu/~boyd/cvxbook/). This is a mathematical
reference, not copied source code. The independent implementation adds no
library dependency and uses no OCCT source.

### Convex operand difference boundary selection

Sequential convex half-space clipping partitions the first operand into common
material and disjoint outside regions. Exterior face fragments inherited from
the first operand, together with oppositely oriented cutter faces bounding the
common material, define the difference boundary. Artificial partition interfaces
are omitted before sewing. Generated arrangements reconcile only floating-point
roundoff bounded by `min(64*EPSILON*local_diagonal, linear/1024)`; independent
patch sewing remains strict. This independently implemented boundary selection
adds no dependency and uses no OCCT source.

### Convex operand union boundary selection

For regular overlapping convex solids, the union boundary consists of each
operand's outward boundary fragments outside the other operand. Independent
half-space partitions provide these fragments; internal partition caps are
removed before sewing. Inclusion-exclusion gives the independent analytic
volume check `V(A ∪ B) = V(A) + V(B) - V(A ∩ B)`. This implementation is original
Rust code under MIT OR Apache-2.0, uses the existing geometry/partition/sewing
infrastructure and adds no dependency. No OCCT source was consulted or translated.

### Axis-aligned box boundary arrangements

The minimum/maximum coordinate planes of two boxes induce a finite rectangular
cell decomposition. Boolean truth functions select material cells; faces between
two selected cells are interior and do not belong to the boundary. Regularization
discards zero-volume face/edge/point intersections. This follows elementary set
operations and orthogonal polyhedron boundary construction, implemented originally
in Rust under MIT OR Apache-2.0. Corners use exact shared input coordinates and
strict planar sewing. No new dependency or OCCT source is used.

### Coplanar face boundary merging

Within an edge-connected component of faces on the same oriented plane, shared
coedges cancel from the exterior boundary. The remaining directed cycles define
outer rings and holes via signed planar area and containment. This is elementary
oriented boundary cancellation and polygon topology, implemented originally in
Rust under MIT OR Apache-2.0. Plane identity is certified by exact dyadic scalar triple products of plane
bases and origin differences; no approximate coplanarity inference is used.
Original affine pcurves undergo checked local frame conversion when needed. This reuses the
existing boundary graph and planar sewing code and adds no dependency. No OCCT
source was consulted, copied or translated.

### Exact 3D orientation and frame conversion

The signed scalar triple product gives tetrahedron orientation and tests whether
a vector lies in a plane span. Exact dyadic integer arithmetic extends the
existing independent finite-binary64 sign implementation to three-factor products.
Plane identity tests use raw bases and exact origin differences, without adding
basis vectors to origins. Affine pcurve frame conversion follows elementary
linear coordinate transformation. Native i128 and WASM BigInt oracles independently
verify determinant signs. This is original MIT OR Apache-2.0 Rust code with no new
dependency and no OCCT source consultation/copying/translation.

### Shared straight edge simplification

A degree-two vertex strictly between collinear endpoints does not change the
straight boundary set. Exact projected orientation signs certify collinearity
in 3D; exact UV orientation and continuity preserve both incident trim wires.
Globally removing eligible knots and rebuilding shared normalized-parameter lines
preserves manifold adjacency and affine pcurves. This is original MIT OR Apache-2.0
Rust code based on elementary straight-line geometry and boundary topology,
reusing existing exact predicates and validation. No new dependency or OCCT source
is used; approximate straightening and healing are not part of this operation.

### Full-cylinder solid classification

Euclidean distance to a bounded full cylinder's lateral surface combines radial
gap and axial interval distance. Circle-trim membership uses analytic radial
comparison. Sorted oriented ray crossings determine regular solid membership;
two independent nondegenerate rays must agree. This extends the existing original
classifier and checked analytic line/cylinder roots, under MIT OR Apache-2.0.
No mesh membership approximation, dependency addition or OCCT source use occurs.
Full periodic walls have no angular trim boundary; the artificial parameter seam
is excluded from physical boundary and crossing-degeneracy tests.

### Diameter-closed half-ellipse trims

An affine ellipse maps to the unit circle; a pi-sweep arc and its closing
straight diameter bound the disk intersected with a half-plane. Line/circle
roots on the retained half and a line/diameter crossing bound each convex
interior interval. Inverting the original affine line pcurve preserves the
owning diameter parameter. Green's theorem gives exact signed area; the existing
Taylor interpolation bound and point-to-segment distance bound physical boundary
proximity. This is independently authored MIT OR Apache-2.0 Rust code using the
mathematical sources already listed for planar ellipse trims. No new dependency
or OCCT source consultation, copying or translation is involved.


### Minor ellipse/chord trims

An affine minor ellipse arc and its endpoint chord map to a unit disk cut by
`dot([cos(s/2), sin(s/2)], q) >= cos(s/2)`. The circular-segment area is
`R^2*(s - sin(s))/2`; symmetry about Y makes the X first moment vanish, giving
the obliquely capped fixture volume directly. The resolved-thickness bound uses
`1-cos(s/2) = 2*sin(s/4)^2` and a lower singular-value bound, avoiding subtractive
cancellation for small angles. Line/circle and finite-chord intersections retain
original parameters. Existing Green's theorem, Taylor interpolation and physical
segment-distance arguments apply on the restricted angular interval. This is
original MIT OR Apache-2.0 Rust code based on the mathematical sources above,
with no new dependencies or OCCT source consultation, copying or translation.


### Concentric homothetic ellipse holes

An affine ellipse annulus maps to concentric circles when the inner and outer
axis matrices differ by one positive scale (or both axes are negated). A lower
singular-value bound maps the radial gap to a conservative physical clearance;
coefficient deviation is bounded by the sum of center and axis-vector errors.
Sorted shared-parameter boundary events subtract the inner convex interval from
the outer interval. Green's theorem integrates CW hole area with opposite sign.
Shared edge orientation includes each face's orientation, so inward tube walls
close consistently with the annular cap. The existing mathematical references
for affine-circle roots, interpolation and ray classification apply to both wires.
This is original MIT OR Apache-2.0 Rust code, with no new dependency or OCCT
source consultation, copying or translation.


### Offset homothetic ellipse holes

An aligned homothetic ellipse maps to a circle of radius k with displaced center
q under the outer ellipse's inverse affine map. The triangle inequality gives
strict containment when `norm(q)+k<1`; multiplying the radial gap by a lower
singular-value bound gives conservative physical clearance. For an oblique cap
`z=-slope*x`, integrate the column height `height/2-slope*x` over outer disk
minus hole. A disk centered at cx has X first moment `pi*r^2*cx`, giving the
fixture's offset-dependent volume. Whole-curve coefficient-error sums and checked
binary64 margins preserve owning 3D edge/pcurve correspondence without snapping.
Existing affine-circle roots and interval subtraction retain the displaced
parameters. This is original MIT OR Apache-2.0 Rust code, with no new libraries
or OCCT source consultation, copying or translation.


### Multiple homothetic ellipse holes

Under the common outer inverse affine map, each aligned homothetic hole becomes
a circle with its own radius and center. Pairwise center distance minus both
radii gives a normalized separation bound. Multiplication by a lower singular
value and subtraction of checked coefficient/arithmetic margins bounds physical
clearance without sampling. Sorting original line parameters and verifying
paired hole events yields the material gaps independent of hole order or line
orientation. Signed circle areas and first moments sum each removed column's
contribution to exact oblique-cap volume. Existing shared topology, ray and chord
bounds apply to all wires. This is independently authored MIT OR Apache-2.0 Rust
code, with no added dependency or OCCT source consultation, copying or translation.

## Unequal-axis ellipse hole certification

Independently derived affine unit-disk normalization, 2×2 Gram-matrix
eigenvalues (spectral norm), enclosing-circle triangle inequality clearance,
and direct oblique plane/cylinder substitution underpin the
[tilted-bore implementation](tilted-bore.md). No new dependencies or OCCT source
were used. Original implementation remains MIT OR Apache-2.0.

## Ellipse supporting-line certificate

The projection radius `hypot(n·A,n·B)` is derived independently by maximizing
a sine/cosine linear combination. Strictly separated projection intervals
certify separation of the convex filled ellipses. A finite direction search
with physical and arithmetic guards is sufficient, not complete. See
[ellipse separation](ellipse-separation.md). No new dependency or OCCT source.

## Full-height nonparallel bore separation

Independently derived horizontal cylinder sections have affine center drift
and constant projection support radii. A same-sign positive gap at both height
endpoints bounds the affine gap throughout the interval. The guarded finite
direction search certifies a separating plane, not a sampled mesh
intersection. See [independently tilted bores](divergent-tilted-bores.md).
No new dependencies and no OCCT source were used.

## Arbitrary-azimuth cylindrical tools

Independent orthonormal-basis rotation gives axis, radial basis, horizontal
ellipse section and harmonic wall height coordinates. The full-height
certificate projects both XY drift components and rotated ellipse basis
vectors, using the same affine endpoint argument. See [oriented bores](oriented-bores.md).
No new dependencies or OCCT source were used.

## Cylindrical blind-bore difference

Independent primitive construction uses a circular cap inner wire, inward
cylinder, upward disk floor and shared seam/rim topology. Removed volume is
`pi*r²*depth`; analytic surface integration and existing B-rep tessellation
verify the result. See [blind bores](blind-bore.md). No new dependencies or
OCCT source were used.

## Six-face blind-bore placement

Right-handed signed coordinate permutations map each chosen box-face outward
normal to canonical +Z. Exact primitive blind construction followed by checked
rigid B-rep placement preserves normals, curves, pcurves and volume. See
[six-face blind bores](box-face-blind-bore.md). No dependencies or OCCT source
were added.

## Editable workflow documents

The operation document is independently designed, with strict typed JSON
encoding using [Serde](https://serde.rs/) and serde_json. JSON syntax follows
[RFC 8259](https://www.rfc-editor.org/rfc/rfc8259). Modeling continues to use
Hagane exact primitives and tolerance checks. These libraries implement
serialization, not geometry. No OCCT source or bindings are used.

Additional locked Rust dependencies (runtime serialization and compile-time
derive tooling):

| Package | Locked version | Declared license | Source |
| --- | --- | --- | --- |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| quote | 1.0.47 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| syn | 3.0.6 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij |

Bundled license files were inspected. The MIT alternative is retained in
[workflow dependency notices](workflow-dependency-notices.txt); unicode-ident
also requires Unicode-3.0, whose full copyright/permission notice is included.
Preserve these notices when redistributing relevant source/artifacts.

The browser distribution also includes [full kernel/dependency notices](../web/third-party-notices.txt),
including existing triangulation dependencies, so these notices accompany web artifacts.

- Planar-subject / convex-tool clipping reuses supporting half-space intersection,
  plane partitions, original-plane retained-face provenance and exact planar
  sewing. Convex solids are intersections of their outward supporting half-spaces;
  see [convex polyhedra](https://en.wikipedia.org/wiki/Convex_polytope).
  Deterministic plane scheduling defers disconnected intermediate shells rather
  than altering geometry. Implemented independently, using no OCCT/reference
  source code and no additional dependency. See [planar/convex Boolean scope](planar-convex-booleans.md).

- Multi-component planar partition groups generated faces through exact shared
  source/intersection vertices (connected-component graph traversal), then sews
  and validates each group independently. No display mesh or tolerance snapping
  defines component membership. Volume conservation follows the existing plane
  partition / divergence-theorem construction. Independently implemented,
  without new libraries or copied reference code.

- Multi-component planar Booleans carry all negative/positive connected solids
  through supporting half-space clipping. Retained subject faces and reversed
  cutter faces are sewn with shared straight-edge adjacency, then extracted into
  independently validated shells. Total-volume identities check partition and
  difference. Only existing bounded arithmetic-roundoff reconciliation is used
  for internally generated patches. Implemented independently without new
  dependencies, copied code or mesh Boolean operations.


- Floating coordinate guards use absolute coefficient envelopes and
  [Rust f64::EPSILON](https://doc.rust-lang.org/std/primitive.f64.html#associatedconstant.EPSILON)
  for binary64 spacing. The conservative factor and quarter-error acceptance
  policy are Hagane's bounded implementation choices, independently tested;
  they are not a copied CAD algorithm or a formal interval-libm proof.
- Stable closed display-mesh volume uses recentered scalar triple products
  (the divergence-theorem tetrahedral formula) and the publicly described
  [Neumaier compensated-sum variant](https://en.wikipedia.org/wiki/Kahan_summation_algorithm#Further_enhancements).
  Implemented independently from these mathematical operations without new
  libraries, OCCT code or translated implementation source.


Rectangular NURBS face topology uses the independently implemented rational
isocurve contraction above, positive UV loop orientation and explicit affine
same-parameter maps. Canonical control/basis checks establish restricted
boundary structure; no global regularity or injectivity theorem is claimed.
Rigid placement acts directly on rational control coordinates. No OCCT code,
external NURBS implementation or new dependency is used.


Single-span rational surface display uses the tensor Bernstein product identity
for `D = X - W*B`, where `B` is the bilinear corner interpolant. Convexity of
coefficient combinations and positive denominator weights bounds `|S-B|`; the
bilinear twist term `|C00-C10+C11-C01|/4` bounds `|B-T|` for two triangles.
These mathematical identities are independently implemented, with local
homogeneous de Casteljau subdivision. Corner mismatch and the engineering
floating-point allowance are separate implementation guards, not a formal
interval arithmetic proof. No reference code, OCCT source, or new dependency
is used; see [bounded surface display](nurbs-surface-tessellation.md).


Tensor-product Bezier extraction raises each interior knot's multiplicity to
its degree using the same independently implemented homogeneous insertion
formula, then restricts controls to each nonzero parameter rectangle. The
Bernstein coefficient bounds above apply per extracted patch; a shared global
subdivision level preserves the tensor grid across nonuniform knot spans.
Original-source evaluation supplies shared positions and C1 seam normals.
Extraction rounding is handled by the engineering arithmetic allowance, not
formal interval certification. No reference code or new dependency is used.


C0 rational surface shading uses the existing analytic one-sided rational
partials independently on each incident parameter cell. A UV-only canonical
node retains the continuous source position while separate side-pair display
vertices retain crease normals. This preserves the preceding geometric bounds
without averaging normals or welding coincident positions at unrelated UV.
Signed-zero parameters are numerically equal and therefore canonicalized for
node/cache keys. These are independent topology/display policies derived from
continuity and one-sided derivatives, with no copied reference code or dependency.

Rectangular rational surface restriction uses homogeneous knot insertion to
raise cut multiplicities, tensor control-net selection and endpoint clamping.
It follows the same publicly described B-spline refinement identities cited
above (de Boor; Piegl and Tiller), independently implemented without copied
source or an additional dependency. Rational geometry is preserved mathematically;
the implementation uses checked `f64` arithmetic rather than symbolic or interval
arithmetic. Boundary topology is regenerated from exact retained isocurves.

Rectangular UV openings use the same independently implemented homogeneous
refinement/isocurve identities to retain exact clockwise inner wires. Inserting
all boundary coordinates as tensor knot lines produces a conforming parameter
grid whose cells can be classified against rectangular trim interiors without
approximate curve clipping. The retained rational surface and trim topology
precede mesh generation. UV separation uses a separate parameter arithmetic
guard, not a physical-length tolerance. No OCCT code or dependency is added.

Material-only rectangular trimming classifies the tensor span grid against
exact retained UV rectangles before subdivision and normal evaluation. All
retained spans share one dyadic refinement depth, preserving prior conformity
and Bernstein bounds while avoiding evaluations in removed interiors. The
singular-source demonstration uses independently derived polynomial-to-Bernstein
coefficients for the displayed formulas, with unit rational weights. These
changes use no copied source, external CAD implementation or new dependency.

Straight UV path composition restricts homogeneous tensor Bezier controls to
each path rectangle, contracts constant axes, reverses axis controls when needed,
and uses the Bernstein product identity
`B_i^p(t) * B_j^q(t) = binomial(p,i)*binomial(q,j)/binomial(p+q,i+j) * B_(i+j)^(p+q)(t)`.
Original knot crossings divide the path into exact mathematical pieces with
common homogeneous weight scaling. Homogeneous endpoint agreement and explicit
numerical guards address `f64` stitching separately from the mathematical
identity. The affine UV pcurve uses the same dimensionless path parameter as
the retained 3D rational edge. This implementation is independent and uses no
OCCT source, copied CAD routine or new library.

### Convex UV surface wires

The supporting-half-plane characterization of a strictly convex polygon is used
with Hagane's independent exact `orient2d` sign predicate: every nonincident
corner must lie strictly on the same side of each oriented boundary edge.
This is a mathematical convexity criterion, not an OCCT implementation or a
borrowed triangulation routine. Lifted geometry uses the Bernstein composition
already recorded above. No new dependencies or third-party code were added;
original implementation remains MIT OR Apache-2.0.

Convex polygon-face display uses independent fan triangulation on the UV
polygon and affine invariance, with an engineering reserve for residual bilinear
twist and coordinate arithmetic. No third-party code or new dependencies were
introduced (MIT OR Apache-2.0 original code).

Bilinear polygon display uses barycentric affine reproduction and the range
bound on the remaining UV product over a triangle bounding rectangle. Shared
edge midpoint subdivision is implemented independently. Floating arithmetic
reserves are engineering checks, not interval proofs. No new dependencies or
OCCT code were used (original code MIT OR Apache-2.0).

Rational bilinear polygon bounds use the quotient identity H=W*S, positive
Bernstein denominator bounds, differentiated homogeneous control differences
and the barycentric Taylor remainder with Hessian bounds. These mathematical
identities are implemented independently with engineering floating guards;
no interval certification is claimed. No new dependencies or OCCT code were used.

High-degree polygon bounds use the standard Bernstein derivative identity
`d B_i^n/dt = n*(B_(i-1)^(n-1)-B_i^(n-1))`, scaled tensor control differences,
positive rational denominator bounds and twice-differentiated H=W*S. The
implementation is independent and retains engineering arithmetic reserves; no
OCCT code or dependencies were introduced (MIT OR Apache-2.0 original code).

C1 multi-span polygon bounds combine independent rational Bezier extraction,
UV chain-rule rescaling of first/second derivative bounds, and the fundamental
theorem of calculus for a continuous piecewise-smooth first derivative. The
implementation does not claim interval certification and adds no dependencies
or OCCT code (original code MIT OR Apache-2.0).

Crease-aware polygon display uses independent convex half-plane clipping of
UV triangles, shared-edge intersection identity, uniform midpoint subdivision
and explicit one-sided source derivatives. No third-party clipping code or
OCCT source was used; engineering f64 guards remain distinct from interval
certification (MIT OR Apache-2.0 original code, no new dependencies).

Convex rational-face openings use exact affine-UV inner wires with clockwise
traversal and independent convex-cell half-plane partition before triangulation.
Source-domain clearance guards are separate from physical-length tolerance.
Material-only normal sampling follows the retained UV region; global bounds
remain conservative engineering checks. No dependencies or OCCT source were
added (original implementation MIT OR Apache-2.0).

Mathematical clipping reference: I. E. Sutherland and G. W. Hodgman,
“Reentrant Polygon Clipping,” Communications of the ACM 17(1), 32–42 (1974),
https://doi.org/10.1145/360767.360802. The half-plane procedure here is independently
implemented; no code from that publication or another CAD kernel was copied.

Closed polynomial graph solids use the quadratic Bernstein basis
`B_1^2(t)=2t(1-t)`. Moving only the roof center control by `b` yields
`4b u(1-u)v(1-v)`; integrating each factor on `[0,1]` gives the exact
volume correction `LWb/9`. A Hessian Taylor remainder bounds linear
triangle interpolation. Shared B-rep edge identities, rather than coordinate
proximity welding, close the display grid. This scoped implementation is
original MIT OR Apache-2.0 code, with no additional dependencies or OCCT source.

Graph-solid placement uses the existing right-handed orthonormal transform
and homogeneous control-point affine invariance. Canonical local controls
are transformed under a composed placement, with unchanged weights, knots
and parameter curves. Absolute transform coefficient envelopes account for
world-coordinate arithmetic, including cancellation. Nonidentity bounds use
the positive-weight control-hull property and are explicitly conservative.
This is independent original code; no new dependency or OCCT source is used.

Graph-solid restriction combines the existing exact knot-insertion surface
restriction with ruled tensor-product walls sharing the retained roof's
boundary controls. Polynomial antiderivatives give exact retained volume;
midpoint integral forms avoid subtracting nearly equal cubic values.
Scaled roof and ruled-wall Hessians bound corresponding-UV triangle errors.
Original parameters and pcurves remain retained, independently of the display
grid. No new dependency or OCCT source is used (MIT OR Apache-2.0).
