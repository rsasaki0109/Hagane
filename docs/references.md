# Provenance, references, and licenses

Normal-prism/convex-tool partition uses independently implemented planar
arrangements, analytic line/line and line/circle roots, directed boundary
classification and cycle reconstruction. Conceptual reference: M. de Berg,
O. Cheong, M. van Kreveld and M. Overmars, *Computational Geometry: Algorithms
and Applications*, third edition, Springer, 2008. Circle-segment area follows
integration of `sqrt(r²-y²)`; extruded volume is exact cap area times height.
Original code is MIT OR Apache-2.0, with no OCCT source or new dependencies.
See [certified operation domain](prism-convex-partition.md).


Multiple normal-prism blind pockets reuse the exact single-cavity construction,
strict projected-disk separation, checked topology remapping and cylinder-volume
conservation. Direct removed totals use standard Kahan compensated summation:
W. Kahan, "Pracniques: Further Remarks on Reducing Truncation Errors,"
Communications of the ACM 8(1), 1965, doi:10.1145/363707.363723.
Original MIT OR Apache-2.0 Rust, no OCCT source or new dependencies.
See [operation domain](normal-prism-blind-bores.md).

Single-cavity analytic STEP certification restores a proof clone of actual
normal stock, validates strict circular-pocket construction, and compares full
affine/trigonometric boundary coefficients and oriented cylinder/plane geometry.
It retains the original imported B-rep. The two-quarter-pocket negative fixture
is generated from two independently constructed disjoint exact cavity boundaries
and validated as a closed solid. Original MIT OR Apache-2.0 code, no OCCT source
or dependencies added. See [certificate domain](normal-prism-blind-step.md).

Normal-prism blind-bore construction uses elementary circle parameterization,
rigid frame coordinate projection, opposite boundary orientations, the
divergence theorem and the analytic cylinder volume `pi*r^2*depth`.
Four-quarter cavity walls and the planar floor retain same-parameter pcurves.
This original MIT OR Apache-2.0 implementation adds no dependencies and uses
no OCCT source. See [scope and validation](normal-prism-blind-bore.md).

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

Source-axis graph partition derives its world plane from the retained rigid
frame and splits the original UV interval. Exact surface restriction and
ruled sections preserve rational boundaries; corresponding Bernstein controls
bound the two cut faces' geometric agreement because their bases and positive
weights match. Analytic volume conservation has a separate floating-point
guard. This is independent original code, not a general plane/solid solver,
and adds no dependency or OCCT source.

Genus-one graph openings reuse exact rectangular cap inner wires and ruled
walls whose upper curves match the retained NURBS basis. Common knot-aligned
tensor cells preserve material boundaries and topological rim identities.
Analytic integration over four disjoint positive strips with compensated
summation avoids subtracting nearly equal source/tool volumes. The Euler
count includes each cap's inner wire. Explicit knot-side derivatives handle
structural refinement knots on the geometrically smooth polynomial graph.
This is independent original MIT OR Apache-2.0 code, with no added dependency
or OCCT source.

Scoped graph-solid point classification uses the positive-weight rational
Bezier convex-hull property for control-box distance lower bounds. Actual
surface evaluations supply upper bounds; adaptive subdivision refines the
Euclidean boundary band. Tangent-plane least-squares steps propose witnesses
only and are not assumed to converge to a global minimum. Canonical analytic
graph membership is used only after separation from retained boundary patches.
This independent MIT OR Apache-2.0 implementation adds no dependency and uses
no OCCT source. Floating-point guards are engineering bounds, not interval
arithmetic certificates.

Source-vertical graph sections use the canonical polynomial roof height at
original UV coordinates and the actual retained cap evaluations and partials.
Rigid frames preserve physical height parameters and outward crossing normals.
The minimum distance from an infinite source-vertical line to a finite vertical
wall is its horizontal distance to the wall's rectangular base segment; endpoint
clamping and hypot preserve Euclidean corner contacts. Section metadata is
checked against the canonical retained construction. This independent
MIT OR Apache-2.0 implementation adds no dependency or OCCT source.

The graph-solid STEP writer follows public ISO 10303 geometry/topology entity
definitions for `b_spline_curve_with_knots`, `b_spline_surface_with_knots`,
`surface_curve`, `pcurve`, oriented face bounds and manifold solids. Original
unit-weight control nets represent polynomial geometry exactly without fitting;
affine UV lines preserve retained parameter maps. Public entity reference:
<https://www.steptools.com/stds/stp_aim/html/>. This independent original
MIT OR Apache-2.0 writer adds no dependency or copied/translated OCCT source.
The optional `occt-import-js` reader remains an external LGPL-2.1 test oracle
with bundled OCCT's LGPL-2.1/OCCT exception, as recorded in
[STEP export](step-export.md); its imported mesh mass is approximate evidence.

The strict graph STEP importer reuses the bounded original Part 21 parser with
a separate allowance for polynomial B-spline curves/surfaces and surface curves.
Control-net identities recognize the canonical graph and retain actual parsed
geometry; only shared reference indices and cyclic wire starts are reordered.
No coordinate fitting or tolerance-based snapping is performed. Public
geometry/topology entity definitions are linked above; this original
MIT OR Apache-2.0 implementation adds no dependency or OCCT source.

The holed graph importer uses the public degree-two polynomial blossom identity
for `2u(1-u)`: control coefficients `s + t - 2st` at successive inner knots,
combined as a tensor product. This follows standard B-spline blossoming and
knot-refinement mathematics described in the de Boor/Piegl–Tiller references
above. A bounded binary64 candidate search is only representation recognition;
all encoded geometry must match exactly. Original MIT OR Apache-2.0 code adds
no dependency and copies or translates no OCCT source.

Scoped graph uniform-density moments use elementary column integrals:
volume density `h`, horizontal moment densities `x*h`, `y*h`, and vertical
moment density `h²/2`. Tensor three-point Gauss–Legendre integration is exact
for each coordinate polynomial of degree at most five; these densities reach
at most degree four. Public mathematical reference:
https://dlmf.nist.gov/3.5#v (Gauss quadrature). Scaled positive material-strip
sums and local-to-world centroid placement avoid subtractive hole moments and
large translated world moments. Floating-point guards are engineering checks,
not interval arithmetic. Original MIT OR Apache-2.0 implementation introduces
no dependency and copies or translates no OCCT source.

Scoped graph centroidal inertia uses the public central-column identity
`integral_0^h (z-cz)² dz = h*((h/2-cz)²+h²/12)`, the parallel-axis theorem
for independent tests, and the rank-two tensor frame transformation `R I Rᵀ`.
Four-point tensor Gauss–Legendre integration exactly integrates degree-six
polynomials in real arithmetic; the public NIST quadrature reference above
applies. Geometric unit-density tensors have length-to-the-fifth units.
Original MIT OR Apache-2.0 code adds no dependency or OCCT source.

Affine graph roof sections use the public Bernstein product identity:
`B_i^n(t) B_j^m(t) = C(n,i) C(m,j) / C(n+m,i+j) B_(i+j)^(n+m)(t)`.
Pullback of the polynomial tensor roof along an affine UV path yields degree
four, with affine XY coordinates elevated to the same degree. Rectangular
opening slab intervals retain the original path parameter. These are elementary
polynomial/B-spline and interval-clipping identities, independently implemented
under MIT OR Apache-2.0 without new dependencies or OCCT source.

Convex polygon graph solids reuse the public rational Bernstein parameter-curve
restriction and ruled-surface construction described above. Positive fan
triangles use the elementary Duffy map
`P=C+r*(A-C)+(1-r)*s*(B-C)`, with Jacobian
`det(A-C,B-C)*(1-r)`. Tensor five-point Gauss–Legendre quadrature
(NIST DLMF 3.5(v), linked above) integrates height-square total degree eight
and its degree-one Jacobian exactly in real arithmetic. Convex combinations
and polynomial Hessian bounds control display error, with explicit binary64
engineering allowances. Original MIT OR Apache-2.0 code adds no dependency
and copies or translates no OCCT source.

Source-vertical graph partitions use elementary convex polygon half-plane
clipping: for opposite signed distances, the edge crossing fraction is
`d0/(d0-d1)`. Shared crossings and canonical curve direction retain identical
cut bases under Bernstein/NURBS parameter reversal. Plane normals follow the
physical scaled UV tangent and its source-XY perpendicular. Polynomial column
moments and positive triangle quadrature reuse the public references above.
Closed convex interpolation in display preserves mathematical endpoint ranges
without changing B-rep geometry. Independent MIT OR Apache-2.0 implementation
adds no dependency and copies or translates no OCCT source.

### Polygon graph material domains

The existing ISC-licensed `earcutr` dependency supplies candidate planar UV annulus
triangulations. Original Rust checks exact orientation, embedding, oriented
incidence and connectivity; bounded convex diagonal flips preserve the same
planar domain without moving vertices. Positive Duffy/Gauss quadrature integrates
polynomial column moments. These use planar geometry and polynomial quadrature
identities; no OCCT source is copied or translated.

### Retained polygon graph point classification

Positive rational Bernstein convex hulls bound actual retained NURBS surfaces.
Evaluated material-only surface points provide upper distance witnesses;
iteration candidates never establish convergence or a closest-point proof.
Planar convex clipping restricts search cells without changing the B-rep.
Original code combines these mathematical identities with bounded subdivision
and explicit engineering precision guards; no external CAD source is used.

### Rational polygon graph STEP export

ISO 10303 spline subtype definitions encode rational curves/surfaces with
Part 21 complex entity components: inherited B-spline basis attributes,
knot data and explicit positive weights. The existing public STEP Tools
entity reference above provides the geometry/topology definitions. Original
Rust serialization retains actual near-unit binary64 weights without fitting
or normalization. The existing optional `occt-import-js` reader is only an
external interoperability oracle under its recorded LGPL/OCCT exception terms;
it is not a kernel dependency and no OCCT source is copied or translated.

### JSON binary64 round-trip parsing

The existing `serde_json` dependency now enables its documented `float_roundtrip`
feature: <https://docs.rs/crate/serde_json/latest/features#float_roundtrip>.
Independent Rust `f64::from_str` and hard-coded IEEE-754 bit patterns verify
reproducible finite numeric input. No dependency version or license changes;
`serde_json` retains its recorded MIT OR Apache-2.0 terms.

### Polygon graph centroidal inertia

The central-column identity and world rotation convention reuse the graph
inertia references above. The triangle Duffy map and positive material
decomposition reuse the polygon moment references. Seven-point Gauss–Legendre
quadrature (public NIST DLMF https://dlmf.nist.gov/3.5#v) integrates through
degree thirteen in each mapped coordinate, including the degree-twelve h³
term and the Duffy Jacobian. Original Pure Rust code; no new dependency,
external CAD implementation source or license change.

### Multiple polygon graph openings

Physical supporting-edge separation reuses convex half-space geometry;
multiply connected planar material regions have Euler characteristic 1-g
and n+2g-2 triangles without Steiner vertices. Exact collinear edge splits,
oriented boundary/interior incidence and nonintersection checks preserve
that embedding. Positive Duffy/Gauss polynomial properties and world inertia
reuse the references above. The existing Rust earcutr dependency remains ISC;
no new library, external CAD source or license change is introduced.

### Multiple polygon graph STEP export

The retained multi-opening B-rep uses the existing public ISO 10303-21/AP214
entity mapping and rational spline references recorded above. Outer and inner
face bounds preserve canonical wire traversal; each shared surface curve has
both incident pcurves. Independent entity decoding checks topology and spline
data. See [multi-opening STEP export](nurbs-graph-polygon-multi-hole-step.md).
Original Rust code; no new dependency, external CAD source or license change.

### Multiple polygon graph point classification

The existing retained-face classification algorithm now excludes the union of
all admitted convex openings. Positive material-region triangles, Bernstein
control-hull distance bounds and actual surface witnesses reuse the polygon
classification references above. Convex physical supporting-line clearances
give a conservative deep-void exclusion before subdivision. Engineering
arithmetic and work guards remain unchanged. Original Rust implementation;
no new dependency, external CAD source or license change.

### Strict polygon graph STEP recognition

The opt-in Part 21 reader uses the public AP214 rational spline component
mapping recorded above, retaining positive weight arrays and spline bases.
Affine uses are accepted only when their serialized origin, direction ratios
and magnitude match the same original writer decomposition of a canonical
representative. This is checked representation recovery, not arbitrary input
vector normalization. Full actual-geometry reindexing and canonical topology
validation remain required. Original Rust code; no new dependency, external
CAD source or license change. See [polygon import](nurbs-graph-polygon-step-import.md).

### Holed graph STEP recognition performance

Canonical recognition reuses the same original graph-roof construction and
rectangular hole knot-insertion routines for an exact surface precheck.
Matching candidates still undergo the existing full geometry/topology
reconstruction and validation. Bounded candidate enumeration and admission
conditions are unchanged. The benchmark uses Rust `Instant` and public kernel
APIs; no dependency, external CAD source or license change is introduced.

### Rational surface/UV-curve composition

The new composition uses homogeneous rational coordinates, tensor-product
Bernstein bases and Bernstein polynomial multiplication from the public
mathematical formulation of Bezier and NURBS geometry (Piegl and Tiller,
*The NURBS Book*, second edition, 1997, as recorded above).
The original Rust implementation substitutes the UV numerator and denominator
polynomials into each surface basis term; it does not copy an external CAD
implementation. Actual positive weights and the original curve parameter
interval are retained. Conditioning guards are engineering checks, not formal
interval bounds. `PCurve::Nurbs` and the roof-circle path demo add no dependency
or license change. Original code is MIT OR Apache-2.0. See
[rational roof-circle paths](nurbs-graph-rational-roof-circle.md).

### Circular graph bores and disk moments

Circular trims use standard rational quadratic Bezier quarters and the
original homogeneous surface/curve composition described above. Ruled wall
controls retain the same weights in both height rows. Typed canonical
validation is original Rust code; no external CAD source was used.

Disk column moments follow polar coordinates with squared radius, the
trapezoidal sum for finite angular Fourier polynomials, and four-point
Gauss-Legendre quadrature (the public NIST DLMF 3.5(v) reference above).
Sixteen angular nodes and four squared-radius nodes integrate the required
degree-twelve polynomial moments in real arithmetic. Separate positive stock
and disk quadratures use checked compensated subtraction. Independent tests
use the classical closed even disk moments
`π r^(2a+2c+2) (2a)! (2c)! / [4^(a+c) a! c! (a+c+1)!]`.
Display interpolation uses rational derivative recurrence from `H = W C`,
homogeneous de Casteljau restriction, graph Hessian bounds and an explicit
circular trim chord allowance. Floating-point guards are engineering bounds,
not interval certificates. Original code is MIT OR Apache-2.0; no dependency
or license change. See [circular graph bores](nurbs-graph-circular-hole.md).

### Rational circular-bore STEP export

The original ISO 10303-21 / AP214 writer uses the public STEP entity semantics
already cited above: `PCURVE`, `DEFINITIONAL_REPRESENTATION`,
`B_SPLINE_CURVE_WITH_KNOTS`, `RATIONAL_B_SPLINE_CURVE` and
`GEOMETRIC_REPRESENTATION_CONTEXT`. Rational UV curves retain their actual
two-coordinate control points, weights and knots in context dimension two;
three-dimensional geometry remains in millimetres. No fitting or mesh export
is used. Independent tests decode the actual entities and compare the retained
geometry and oriented shared topology. This adds no dependency or license change.

### Circular-bore point classification

The original positive rational Bezier hull distance search reuses the checked
point-witness/Newton implementation described above. Convexity of a physical
disk justifies discarding an axis-aligned parameter rectangle only when all
four physical corners are strictly inside it. Every retained cap distance
witness is independently filtered; actual ruled spline walls cover circular
rims. Supporting-plane and radial deep-void separation are Euclidean lower
bounds. These engineering arithmetic guards are not interval certification.
No CAD source or new dependency is used; original code remains MIT OR Apache-2.0.

### Editable graph workflow

The versioned graph operation document reuses Hagane's original typed graph,
rational circular-bore, mass, bounded display, classification and STEP APIs.
Immutable prefix snapshots and transactional replay are original application
code; saved data records intent rather than supplying geometry. Existing serde
and serde_json dependencies retain their recorded MIT OR Apache-2.0 terms.
No external CAD source or additional dependency is introduced.

### Isolated planar edge chamfer

The implementation uses elementary supporting-plane half-space intersection
and the Euclidean angle-bisector identity: for outward unit normals n1,n2,
the inward offset is d*|n1 cross n2|/|n1+n2| for equal face setback d.
Hagane's existing original planar splitter supplies closed B-reps. No external
CAD implementation or new dependency is used. Original code remains
MIT OR Apache-2.0; existing dependency licenses are unchanged.

### Transverse multiple planar chamfers

Successive intersections of convex negative half-spaces produce the retained
material; each positive split of the current retained body produces removed
material disjoint in its interior from preceding removals. This uses Hagane's
original planar splitter and bisector geometry. The independent adjacent-wedge
overlap oracle integrates `(a-t)*(b-t)` over `0..min(a,b)`. No new dependency
or external CAD source is introduced; original code remains MIT OR Apache-2.0.

### Vertex-contact planar chamfers

Indexed convex polygon clipping, a shared edge-intersection cache and directed
section-cycle extraction are original code. Conditioning checks bound
along-edge displacement using normal-gap allowances and edge slopes, including
a denominator allowance for binary64 roundoff. Exact binary64 orientation uses
Hagane's existing original integer predicates. The all-edge box volume oracle
uses inclusion-exclusion: each corner's three pair overlaps sum to d³ and its
triple overlap is d³/4, producing the 6d³ total correction over eight corners.
No external CAD source or new dependency is introduced. Original code remains
MIT OR Apache-2.0; engineering guards are not interval certification.

### Parallel box-edge fillets and bounded analytic STEP

The implementation is original code using elementary circle tangency, the
quarter-disk area pi*r²/4, rectangle extrusion and the circular chord sagitta
r*(1-cos(a/2)). Recognition uses actual cube incidence and a checked rigid
frame; it does not borrow another CAD kernel's box or fillet code. The existing
original mixed-profile extrusion supplies shared analytic B-rep boundaries.

The opt-in writer follows the public ISO 10303-21/AP214 entity model already
referenced above: CIRCLE, EDGE_CURVE, SURFACE_CURVE, PCURVE, SEAM_CURVE and
CYLINDRICAL_SURFACE. Bounded edge endpoints and oriented circle geometry retain
the supported circular span. Invalid negative/long canonical arcs and unsupported
imports are not advertised as implemented. No new dependency or OCCT source is
introduced. Original code remains MIT OR Apache-2.0; existing licenses remain
unchanged.

Optional bounded-fillet STEP interoperability checks use the already recorded
cached `occt-import-js`/OCCT reader as an external test oracle. Its existing
license conditions apply; it is neither linked into Hagane nor a source for
the original fillet or writer implementation.

## Bounded analytic STEP reader

The dedicated reader uses the public ISO 10303 entity model referenced above
for oriented topology, CIRCLE, SURFACE_CURVE/SEAM_CURVE and PCURVE. The original
normal-extrusion certificate compares analytic entire line/arc boundaries,
translated caps and shared walls; it retains imported geometry. Plane UV
coordinates scale as lengths; cylindrical U remains angular and V scales as
a length. Conservative floating-point reserves are engineering guards, not
interval arithmetic certification. No OCCT source or new dependency is used.
Optional interoperability checks reuse the previously recorded external OCCT
reader and its license conditions, solely as a test oracle.

The multi-wire analytic prism extension reuses the original line/arc-region
containment and segment-intersection algorithms described above. It checks
whole imported boundaries and corresponding outer/inner wires against that
validated region; no STEP source shape is regenerated as the imported result.
This extends the same public entity model and introduces no new dependency.

## Normal circular prism bore

The original bore implementation reuses the checked analytic line/arc-region
extrusion and region-intersection algorithms, and the whole-curve normal-prism
certificate. Four exact circular quarters describe the opening; cylinder
volume is the classical `pi * r² * h`. Retained curve correspondence and separate
removed-volume/conservation budgets guard reconstruction. No mesh Boolean,
OCCT source or new dependency is introduced; existing licenses remain unchanged.

The rounded-stock workflow composes the already documented analytic parallel
edge fillet and normal arc-line prism bore operations. Its volume check uses
rounded-rectangle area `WD − (4 − π)r²` and cylinder volume `πR²H`; no new
dependency or OCCT source is introduced. Operation documents and prefix caches
are original Rust/JavaScript code under MIT OR Apache-2.0.

The line/arc workflow reuses the existing mixed-profile Green-area integration,
normal-prism certification and analytic normal-bore operations. The independent
capsule check uses rectangle area `2R * center_distance` plus disk area `πR²`,
then subtracts actual cylinder volumes. Its serde document and cache integration
are original MIT OR Apache-2.0 code; no new dependency or OCCT source is used.

## Axial-plane partition of analytic prisms

The normal line/arc prism partition composes the existing analytic planar trim
intersection, shared-edge face subdivision and exact normal extrusion algorithms.
Whole source line coverage and circular subarc basis/interval coverage are checked
independently of child volume. Circle-segment test oracles integrate
`sqrt(r*r-x*x)` using `(x*sqrt(r*r-x*x)+r*r*asin(x/r))/2`; cap areas use the existing
Green-theorem line/arc integrals. No OCCT source or new dependencies are used.
Original implementation remains MIT OR Apache-2.0.

Editable analytic-stock plane cuts compose the independently implemented prism
partition and normal-bore algorithms above. JSON stores plane angle/offset/side
and the original operation chain; reconstruction uses actual accepted B-rep
prefixes. Rounded-half and subsequent bore oracles use the existing
`width*depth - (4-pi)*radius^2` cross-section area and `pi*r^2*height` tool volume.
No new dependencies or OCCT source are used; licensing is unchanged.

Multi-component normal line/arc partition composes the existing oriented planar
cut graph, analytic Green-theorem cap integrals and exact normal extrusion.
Circle-opening subtraction uses the independent circular-segment formula
`r*r*acos(d/r) - d*sqrt(r*r-d*d)`. All resulting regions retain their own closed
topology. Complete source-curve coverage, paired opposing actual cut walls,
per-child area conditioning and compensated total volume summation supplement
construction. This original code adds no dependencies and remains
MIT OR Apache-2.0; no OCCT source was copied or translated.

## Explicit-axis normal-prism continuation

The additive all-line domain reuses the existing actual planar-prism
correspondence certificate and normal line/arc cap, wall and curve-coverage
proofs. Explicit axis selection uses vector projection and bounds transverse
translation physically; no axis is inferred from a box's face ordering. Bore
volume uses pi times radius squared times actual axis height. Cap partition
uses the same published analytic line/circle intersection and Green-theorem
methods recorded above. No OCCT source or new dependency is used.

## Normal stock cuts in editable history

Box/polygon cut histories compose the existing normal-prism bore and analytic
component partition algorithms. Axis selection and actual cap/wall/whole-curve
certificates are unchanged. Incremental reuse includes the construction-domain
identity because full circles and four quarter arcs have different exact
topologies even when modeling intent is equal. STEP dispatch follows the same
identity. No new mathematical method, dependency or OCCT source is introduced.

## Scale-aware perspective display

The display camera uses the elementary right-triangle tangent-sphere relation
`distance = radius / sin(half_fov)` and the standard perspective frustum
projection, with an engineering fit margin. WebGL floating-point attributes
follow the [WebGL specification](https://registry.khronos.org/webgl/specs/latest/1.0/).
Dividing display coordinates by a common finite scale is a similarity transform;
this changes no CAD coordinates or mathematical modeling algorithm. The
implementation and independent projection tests are original MIT OR Apache-2.0
code, with no new dependencies and no OCCT source.

## Multiple-pocket STEP certification

The [batch blind-pocket reader](normal-prism-blind-bores-step.md) uses the same
elementary affine/trigonometric whole-interval identities and oriented B-rep
incidences as the single-pocket certificate. It restores actual source topology
only in a proof clone and replays the existing disjoint-pocket constructor,
using a bijective geometry/topology witness while retaining imported geometry.
Original MIT OR Apache-2.0 Rust; no OCCT source and no new dependencies.

Planar display repair uses dominant-axis projection of already evaluated world
points, existing filtered exact orientation predicates and oriented boundary
incidence. It changes connectivity only when a UV triangle collapses in world
coordinates; original samples, pcurves and chord sampling remain unchanged.
Existing Earcut dependency and license notices are unchanged.

### Finite circle/circle prismatic Boolean arrangements

The additive normal line/arc prism Boolean uses the planar arrangement and
Green-theorem references above. Circle/circle roots follow elementary Euclidean
intersection geometry: the distance along the center line and factored triangle
height determine two candidate points, then finite signed arc intervals select
the actual crossings. Independent two-disk lens tests use sector areas minus
the center triangle, with Heron’s factored area formula. Directed outside
boundaries give the union, inside boundaries the common material, and reversed
tool-inside boundaries the difference. The implementation is original
MIT OR Apache-2.0 Rust code; no dependency or OCCT source was added.

### Holed material regions and containment forests

The normal-prism region Boolean extends the directed planar arrangements above
to both operands’ outer and inner rings. Clockwise inner boundaries exclude
material; classifying fragments against the complete region preserves central
islands and annular tools. Nested output cycles assign each hole to its immediate
containing material boundary, using analytic point-location witnesses rather
than a possibly exterior centroid. References remain the published planar
arrangement and Green-theorem sources above. Original code is MIT OR Apache-2.0;
no library dependency or OCCT source was added.

### Rational conic ruled frusta

The exact four-quarter frustum uses rational conic and ruled-surface constructions
from the Piegl/Tiller NURBS references above: weights [1, sqrt(1/2), 1] describe
each circle quarter, and linear interpolation of coaxial radius/height supplies
a degree [2,1] rational tensor patch. The same rational primary parameter is
retained by planar cap pcurves and affine side UV uses. Polynomial disk moments
give volume/centroid/inertia; Bernstein homogeneous residual bounds certify
triangle/chord approximations and the shared cap boundary. The implementation
is original MIT OR Apache-2.0 Rust code with no copied OCCT source or new library.

### Finite frustum point distance

Frustum point queries project onto a finite side segment in the radius/height
meridian and compute distance to the finite cap disks. Matching query azimuth
minimizes rotational-surface distance; segment endpoints and disk boundaries
include the circular rims. This elementary Euclidean construction is original
MIT OR Apache-2.0 Rust code, without a mesh query or new dependency.

### Axial restriction of rational frusta

A degree-one direction with equal homogeneous weights restricts by affine
interpolation of its control rows. Matching positive rational basis functions
bound the entire physical surface difference by the largest control difference;
finite samples alone are not used as a restriction certificate. Axial partition
uses this published rational/Bernstein basis property with the conic references
above and polynomial disk moments for conservation. Original code remains
MIT OR Apache-2.0; no new dependency or OCCT source is used.

### Exact rational frustum STEP recognition

The scoped reader uses the same ISO 10303-21/AP214 representation conventions
as the original writer. Parsed rational bases and controls are retained, while
normalized LINE/VECTOR and affine UV records require exact preimages of the
writer's scalar operation order before canonical parameter restoration. This
is a representation certificate, not curve fitting or repair. The decoder and
recognizer are original MIT OR Apache-2.0 Rust code; no OCCT source or new
dependency is used.

The separate translated recognizer obtains radii from retained local rational
UV coefficients and height from represented cap separation. Its exact full-body
certificate avoids subtractive world-coordinate radius fitting; it does not
claim to recover unobservable source construction parameters. This extension
uses original MIT OR Apache-2.0 code and adds no dependencies.

The separate cardinal-frame recognizer restricts cap placement directions to
right-handed signed coordinate bases. The 24 bases are the signed permutation
matrices with determinant +1; cross products reconstruct the omitted plane
axis exactly in this domain. This avoids assuming a floating-point round-trip
for arbitrary rotations. Complete representation certificates remain required.
The extension is original MIT OR Apache-2.0 Rust code with no new dependencies.

Rational frustum workflow transactions reuse the original typed rational B-rep,
axial partition algorithms, cardinal STEP reader and immutable Rust `Arc` snapshots.
Volume accumulation uses the compensated summation reference above. This
MIT OR Apache-2.0 workflow adds no dependencies or OCCT source. See the
[document and validation scope](nurbs-frustum-workflow.md).

Axis-angle frustum workflow placement uses the existing independently
implemented Rodrigues rigid rotation and checked orthonormal frames. Components
retain the original typed rational B-rep and source-local partition algorithms;
world inertia uses the standard tensor transformation `R I Rᵀ` and parallel-axis
theorem. Original code is MIT OR Apache-2.0 with no new dependencies or OCCT
source. See [pose scope](nurbs-frustum-workflow.md#axis-angle-stock-placement).

Finite frustum segment intersection derives the implicit conical side equation
and finite disk cap intersections directly. Stable quadratic roots use Vieta's
formulas and algebraic rationalization to avoid subtractive cancellation.
Floating-point allowance design follows standard error analysis concepts in
N. J. Higham, *Accuracy and Stability of Numerical Algorithms*, second edition,
SIAM, 2002; these are engineering guards, not formal interval certification.
The convexity of the Euclidean norm proves the positive affine-radius shortcut.
Original MIT OR Apache-2.0 code retains actual B-rep face UV witnesses and adds
no dependencies or OCCT source. See [query domain](nurbs-frustum-segment.md).

Frustum infinite line/ray queries reduce the domain by a conservative expanded
local AABB. Slab clipping intersects the three coordinate inequalities of this
box; a triangle-inequality bound limits unit-speed travel before clipping.
Original nonunit parameter recovery uses elementary norm scaling. This original
MIT OR Apache-2.0 extension retains the segment solver's actual rational face
witnesses and adds no dependencies or OCCT source. See
[finite-body line/ray scope](nurbs-frustum-line.md).

Closed oblique frustum sections solve the plane equation in the linear ruling
parameter, then cancel homogeneous terms to retain quadratic spatial conics.
Bernstein multiplication by the edge parameter and degree elevation give the
cubic same-parameter UV curve. Positive-weight convex hulls certify whole-curve
cap clearance and bound the plane residual from spatial controls; the
coefficient cancellation identity bounds correspondence to the source patch.
These use the existing public NURBS mathematics references. Original code is
MIT OR Apache-2.0 with no new dependencies or OCCT source. See
[closed section scope](nurbs-frustum-plane-section.md).

Oblique frustum splitting joins the original and section homogeneous curves by
a rational linear ruling. The generator UV inverse follows directly from its
endpoint weights. Planar cut pcurves use affine projections of actual controls;
equal positive weights bound the complete projection residual. Analytic volume
uses ellipse area and cone volume `area*height/3`; rationalizing the difference
of supporting cone volumes gives the stable zero-taper limit documented in
[oblique split scope](nurbs-frustum-plane-split.md). Conforming display reuses
the established rational Bernstein cell/rim bounds with weighted generator
parameters. Original MIT OR Apache-2.0 code adds no dependencies or OCCT source.
