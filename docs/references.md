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
