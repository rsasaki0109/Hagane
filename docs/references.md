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
- Rational B-spline formulas and homogeneous evaluation: Les Piegl and Wayne
  Tiller, *The NURBS Book*, second edition (1997); Carl de Boor, *A Practical
  Guide to Splines*. Mathematical background also appears in
  [de Boor's algorithm](https://en.wikipedia.org/wiki/De_Boor%27s_algorithm) and
  [B-spline derivative expressions](https://en.wikipedia.org/wiki/B-spline#Derivative_expressions).
  Hagane implements these formulas independently; no book/page source code or
  prose was incorporated, and no additional NURBS library is linked.
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
