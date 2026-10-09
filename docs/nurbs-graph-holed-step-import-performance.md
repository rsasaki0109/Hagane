# Holed graph STEP import recognition cost

The strict rectangular-opening STEP importer now tests a coefficient
candidate's roof surface before constructing its complete graph B-rep. This
changes the work spent on rejected candidates while retaining the existing
exact acceptance checks. It adds no dependency or license change.

## Exact recognition remains mandatory

The supported input is still one AP214 polynomial graph body with source
origin zero, identity placement, positive world axes and the full `[0,1]²`
source UV domain, with one strictly interior rectangular through opening.
It retains 16 vertices, 24 shared edges, 10 NURBS faces and annular caps.
SI millimeter/metre coordinates are converted to millimeters; source UV and
knots are not length-scaled. Placed/restricted bodies, multiple or polygon
openings, nonunit rational weights and arbitrary NURBS shells remain outside
this importer. See [strict holed STEP import](nurbs-graph-holed-step-import.md).

The bounded coefficient search and exact XY footprint checks remain in force.
For each candidate, the new surface-only prefilter reproduces the canonical
roof through the same construction/refinement arithmetic and compares actual
degrees, control points, knots and weights. A mismatching surface cannot supply
a complete canonical certificate, so its full solid need not be built.
A matching surface only permits the existing full B-rep construction to run.

The reader still reindexes the actual parsed geometry and then validates every
curve, surface, vertex, affine pcurve, wire, reference and orientation. The
returned B-rep remains the parsed geometry. Surface agreement alone never
accepts a document or replaces its coordinates. Candidate order and bounded
coefficient recovery, parser limits, tolerance checks, unit conversion and
explicit failures retain their existing semantics. There is no fitting,
coordinate snapping or tolerance relaxation.

## Measurement method

```sh
cargo run --release --locked --example benchmark_nurbs_graph_holed_step_import
```

The benchmark repeats each import three times. It prepares the STEP input
before timing and measures import only, including parsing, exact recognition
and internal canonical validation. Separate post-import validation and exact
STEP re-export checks are untimed. STEP export, display meshing, mass properties, process
startup and compilation are outside the timed import region. Compare the same
fixture and build profile before and after the change; retain individual
samples rather than treating a short local run as a universal speed claim.

The fixed native fixtures use dimensions `[80,60,20]` mm, one rectangular
opening `[[0.35,0.65],[0.35,0.65]]`, and roof control offsets `-12`, `0.001`,
`30` and `36` mm, with `Tolerance::default()` (`1e-8` mm). The example defaults to three repetitions and accepts an
optional repetition count from 1 through 100. Its JSON records individual
samples, median, minimum, compiler, architecture, profile and exact re-export
verification.

WASM timing measures the holed-import transport’s `finish` call with a
0.2 mm display request. This includes strict import, canonical reconstruction,
display, mass/inertia and JSON production. Byte pushes, JSON decoding and
browser rendering are outside that interval. It is neither an import-only
measurement nor a complete browser interaction time.

Native and WASM measurements describe their own execution environments.
They should not be compared as equivalent processors or build modes. Timing
can vary with compilation profile, runtime, concurrent work and machine load.
Acceptance/rejection and exact retained geometry are checked separately from
elapsed time; a lower cost does not relax the geometry certificate.

## Recorded before/after measurements

Both native runs used Linux x86_64, a release build and
`rustc 1.99.0 (b940084d7 2026-09-28)`. Each row records three import-only
samples in milliseconds. Every imported part validated and re-exported
byte-identical STEP text outside the timed interval.

| Roof offset (mm) | STEP bytes | Before samples (ms) | Before median (ms) | After samples (ms) | After median (ms) |
| --- | ---: | --- | ---: | --- | ---: |
| -12 | 34,915 | 14.667978, 13.366329, 14.249929 | 14.249929 | 10.076274, 15.051170, 17.913377 | 15.051170 |
| 0.001 | 35,284 | 11.853401, 12.406904, 11.097343 | 11.853401 | 19.540075, 8.413161, 12.104625 | 12.104625 |
| 30 | 35,545 | 11.027048, 16.139893, 9.939111 | 11.027048 | 16.637838, 8.729677, 8.302825 | 8.729677 |
| 36 | 35,252 | 17.278197, 15.858721, 18.667457 | 17.278197 | 15.291472, 20.375303, 16.086789 | 16.086789 |

Native medians increased in two fixtures and decreased in two. The after run
shared the environment with CPU-intensive full native tests; scheduling and
load were uncontrolled. Three samples do not establish a uniform native gain.

The broader WASM `finish` call was measured separately on the same exact STEP
fixtures, with three successful samples per fixture. Retained export bytes
were checked unchanged. Times below are milliseconds and include the broader
work described above, with no browser rendering.

| Roof offset (mm) | Before samples (ms) | Before median (ms) | After samples (ms) | After median (ms) |
| --- | --- | ---: | --- | ---: |
| -12 | 206.743221, 151.469486, 147.510314 | 151.469486 | 162.954081, 133.019489, 127.320121 | 133.019489 |
| 0.001 | 42.476769, 38.763568, 45.463797 | 42.476769 | 31.958549, 32.388235, 32.940778 | 32.388235 |
| 30 | 140.238779, 128.285067, 138.657660 | 138.657660 | 125.384779, 124.798676, 123.590368 | 124.798676 |
| 36 | 481.334058, 444.733629, 414.909611 | 444.733629 | 452.877838, 411.059379, 401.210906 | 411.059379 |

Observed WASM `finish` medians were 7.6–23.8% lower in these four fixtures.
This is local measured latency for import plus reconstruction/display/properties/
JSON, not an import-only result or a claim about an entire browser interaction.
The small `0.001` mm roof coefficient was not the slowest baseline fixture.
Three samples in a shared environment do not establish a universal speedup or
identify the complete browser bottleneck.

The measured module SHA-256 values were:

- Before: `5dd77232c9b5a874185dc612395149e308891c6ae05ac75a20134c39f439a579`.
- After: `47a547a7701588cb60a5dde03fd37184aadece863c94513a000c5fa9daee8bab`.

## Integration and remaining scope

A public-API probe reproducing the unchanged bounded candidate enumeration
found 3, 2, 2 and 4 valid candidate solids before the first exact match for
the four coefficients above. The precheck reduces candidate full-body
constructions to one per fixture. These counts exclude the unchanged flat
prototype and subsequent canonical validation rebuilds. They demonstrate
reduced construction work, rather than a general wall-clock speed guarantee.

`import_step_nurbs_graph_holed_mm` and the common typed graph STEP dispatcher
continue to expose the same APIs. Native and WASM use the same Rust recognition
path, and rejected browser imports preserve the last accepted model. Existing
round-trip and malformed-input regressions retain their exact geometry and
rejection expectations. The plain six-face importer has its separate scope.

This change addresses representation-recognition cost. It does not add
polygon or multiple-opening STEP import, placement/restriction recovery,
arbitrary rational-shell import or general curved Boolean operations.
The original implementation remains MIT OR Apache-2.0 Rust, with the existing
mathematical and STEP references; no OCCT source or runtime dependency is added.

Final native validation passed **708 tests**, formatting and strict Clippy.
The release WASM build, complete WASM regression and full browser regression
passed. The measured WASM fixtures above all succeeded
with unchanged exact STEP re-export; these timing checks are separate from the
complete WASM/browser regression suites.
Mathematical and implementation provenance is recorded in
[references](references.md).
