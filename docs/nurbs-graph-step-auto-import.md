# Common STEP entry point for the supported NURBS graph family

`import_step_nurbs_graph_auto_mm` reads either a plain graph solid or a graph
with one rectangular through opening. Callers no longer have to inspect STEP
text to choose an importer. Its `ImportedNurbsGraph` result keeps the original
checked typed certificate, rather than returning a generic `Solid` whose
analytic-only methods cannot operate on these NURBS faces.

```rust
use hagane::*;
let tolerance = Tolerance::default();
let source = NurbsGraphSolid::new([80.0, 60.0, 20.0], 30.0, tolerance)?;
let step = source.export_step_mm(tolerance)?;
let body = import_step_nurbs_graph_auto_mm(&step, tolerance)?;
body.validate(tolerance)?;
let volume = body.volume()?;
let mesh = body.tessellate_bounded(0.2, 65_536, tolerance)?;
assert_eq!(body.export_step_mm(tolerance)?, step);
match &body {
    ImportedNurbsGraph::Plain(graph) => assert_eq!(graph.dimensions(), [80.0, 60.0, 20.0]),
    ImportedNurbsGraph::Holed(graph) => println!("Opening: {:?}", graph.hole()),
}
# Ok::<(), hagane::Error>(())
```

## One parse, complete validation

The bounded Part 21 decoder runs once. Six actual shell faces select plain
recognition; ten select single-opening recognition. These counts only select
the appropriate full geometry certificate: they never prove validity. Every
control point, knot, UV use, edge orientation and shared reference must still
match. Failures propagate directly, without retrying a rejected shape as a
different type. Product names and filenames do not select a kind.

The enum dispatches `brep`, `validate`, `volume`, `bounds`,
`tessellate_bounded`, `classify_point`, `vertical_section` and `export_step_mm`
to their existing typed implementations. There is no generic shell evaluation,
new geometry recognition or tolerance relaxation. The original plain-only
and holed-only import functions remain strict and reject the opposite kind.
The public enum variants allow access to typed operations; any altered public
B-rep must still pass the original certificate checks.

## Scope and native/WASM use

Supported shapes are the union of the [plain](nurbs-graph-step-import.md) and
[one-opening](nurbs-graph-holed-step-import.md) subsets: full source UV, identity
placement, canonical polynomial bases, mm/metre coordinates. General rational
shells, placed/restricted imports and multiple openings remain unsupported.
The generic analytic STEP importer keeps its own separate scope.

```sh
cargo run --quiet --locked --example nurbs_graph_step_holed_roundtrip > graph.step
cargo run --quiet --locked --example nurbs_graph_step_auto_import -- graph.step 0.2
bash scripts/build-web.sh
node scripts/test-wasm.mjs
```

The common display JSON retains the existing corresponding plain/holed schema
and records `import.kind` as `plain` or `holed`. WASM exposes a separate safe
`hagane_graph_auto_step_import_begin`, `push_byte`, `finish` transport with a
1 MiB limit and UTF-8 validation. Existing browser pages and typed transports
continue to use their strict kind-specific importers.

Native regression tests check both variants' exact STEP re-export, analytical
volume, bounds, genus and material/void classifications and sections. Corrupt
references/pcurves and unsupported placement/restriction are rejected; existing
typed and generic importers remain strict. WASM compares four complete native
display reports, validates retained geometry and topology, and checks UTF-8
failure and subsequent recovery.

The implementation reuses the existing independent MIT OR Apache-2.0 parser
and recognition mathematics, adds no dependency, and copies or translates no
OCCT code. Sources and licenses remain in [references](references.md).
