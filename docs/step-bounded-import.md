# Bounded analytic STEP import

![Actual imported fillet STEP rendered by the Rust/WASM kernel](step-bounded-import.png)

The dedicated `import_step_bounded_analytic_mm(input, tolerance)` reader imports
actual analytic geometry and oriented shared topology. It retains both surface
pcurves and validates the resulting solid. Display meshes are generated from
that B-rep, without mesh Booleans or substituting a generated primitive.

```sh
cargo run --example step_bounded_import -- part.step
# No argument imports a STEP file generated from the actual fillet operation.
cargo run --example step_bounded_import
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
```

Open `step-bounded.html` in the local development server. The browser supports
text and file input, orbit/zoom, and canonical millimetre STEP downloads.
Rejected input preserves the last accepted body and export. Native and WASM
use the same kernel. The demo fixes linear tolerance at 1e-6 mm and display
chord tolerance at 0.1 mm; larger models may need a larger native tolerance.

```rust
use hagane::{import_step_bounded_analytic_mm, Tolerance};
fn read_part() -> Result<hagane::Solid, Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string("part.step")?;
    let tolerance = Tolerance::new(1e-6)?;
    let solid = import_step_bounded_analytic_mm(&input, tolerance)?;
    solid.validate(tolerance)?;
    Ok(solid)
}
```

## Supported domain

The reader supports admitted planar/full-circular bodies with explicit
SURFACE_CURVE/SEAM_CURVE associations and adds
a simple line/circular-arc profile extruded normally between parallel planar
caps. Arc-bearing caps must have one outer wire and no inner wires. Actual
cap translations, entire boundaries, wall geometry and shared incidences must
agree within reserved error budgets. Quarter-circle box fillets are supported
in all three axis families and at resolved rigid placements.

Canonical circular arcs start at their circle placement's zero-angle vertex
and have positive sweep at most pi. Negative master edge sense, long arcs,
arbitrary phase rebasing, freeform curves/surfaces and general STEP assemblies
are unsupported. Exact half-circles at poorly conditioned placements may be
rejected rather than snapping a negative endpoint phase. Partial-cylinder
trims require resolved zero-origin axial parameters. Existing generic readers
retain their original, narrower domains.

Length units are converted to millimetres, including planar pcurves and
cylindrical axial coordinates; cylindrical angular coordinates remain radians.
Invalid/ambiguous owners, missing pcurves, malformed topology, unsupported units
and unresolved geometric precision return errors. Limits include 1 MiB input,
4096 vertices/edges, 512 faces and 4096 total coedges; arc-prism admission has
stricter structural limits.

Canonical export preserves accepted geometry, not original record numbers,
ordering or file bytes. The JSON report explicitly states this distinction.
Display error bounds describe mesh approximation to analytic supporting
surfaces with conservative arithmetic reserves, not a general symmetric
Hausdorff certificate for arbitrary trimmed solids.

## Validation

Native tests cover actual fillet round trips, placements, units, full circular
bodies, short and near-half-circle arcs, coherent geometry tampering, precision
limits, cyclic boundary-loop start rotations and explicit rejection of inner-wire
arc prisms. WASM and browser checks
exercise actual input, shared topology, display bounds, export and error recovery.

Native/WASM parity compares structure, topology and nonnumeric STEP tokens
exactly. Floating values allow 32 machine-epsilon units relative to
`max(1, |a|, |b|)` because platform transcendental functions can differ in their
last bits. Independent geometric, volume and chord checks remain separate.
Canonical native and WASM STEP text is therefore not promised byte-identical.

A separate actual full-cylinder smoke check (radius 12 mm, height 20 mm)
retained three faces, three edges and closed topology. Its volume was
`pi * 12² * 20`, full-circle witnesses reached `2*pi`, and the 96-triangle
display's maximum reported surface error was about 0.094624 mm at 0.1 mm.

The final source passes `cargo fmt --check`, strict all-target clippy, all
840 native tests, the wasm32 release build and the complete WASM regression
suite. Its importer checks include twenty actual fillet documents and their
canonical re-exports, independent geometry/closure/chord checks, metre units,
record and cyclic-loop reordering, and malformed-input transport recovery.

The complete browser regression suite also passes on this final build,
including file/text upload, native numerical parity, canonical download,
accepted-body/export/pixel preservation after rejection, recovery, orbit
and mobile layout. The image above was captured from the actual tested demo.
