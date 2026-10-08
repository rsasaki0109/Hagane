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
