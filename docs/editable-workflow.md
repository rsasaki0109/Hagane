# Explainable editable box-and-bore workflow

![Actual editable operation-history demo](workflow.png)

Open `web/workflow.html` after building the WASM module, or choose **Edit model**
from the main demo. Change width, length, height, tool center, radius and blind
depth. The same pure Rust kernel rebuilds and validates the exact B-rep. Switch
between a box alone, a through bore and a top-entry flat-bottom blind bore.

When a tool reaches a side or breaks through the floor, the report identifies
its operation ID and relevant parameters, measures the clearance, states the
required tolerance margin and suggests an appropriate parameter change. Cyan
outlines mark the rejected candidate tool. They are a display aid, not the result
of a CAD operation. The previous validated solid remains visible, explicitly
labeled **previous valid result**; no replacement mesh is returned as success.
Correcting the edit rebuilds the model without discarding the editable history.

![Actual rejected edit with clearance and candidate tool](workflow-failure.png)

## Save and rebuild modeling intent

**Download editable model** saves the current validated operation document.
**Open saved model** or **Load document** reconstructs and validates the exact
shape, then restores its dimensions and operation IDs. Invalid imports retain
the prior valid result and report the failure. Download is disabled after a
rejected edit; typing into the document editor requires explicit **Load document**.
The exported document stores modeling intent, not mesh geometry or a B-rep
interchange representation. The kernel constructs the B-rep again on load.

```sh
./scripts/build-web.sh
python3 -m http.server 8000 --directory web
cargo run --example workflow -- docs/workflow-example.json
```

The native library exposes `WorkflowDocument::rebuild()` for the exact solid
and `evaluate_workflow_json(text)` for a serialized report. The default
[operation document](workflow-example.json) is independently usable by both
native and WASM. Angular/relative/linear tolerance policy is stored and checked;
the supported axis-aligned primitives currently use its linear component.

## Version 1 domain

- Explicit `schema_version: 1` and `units: "mm"` are required. Other versions
  or units are rejected; values are not silently converted.
- History has one centered axis-aligned `box` operation followed by at most
  one `bore`. Bore input references the preceding box ID; IDs are unique and
  contain 1–64 ASCII letters, digits, hyphens or underscores.
- Box size is `[width, length, height]`. Bore center is world XY in mm.
  `mode: "through"` has no depth (null is treated as absent), while `"blind"`
  requires finite positive depth entering the top/+Z cap. Through tools are
  generated with strict cap overhang, not inferred from blind depth.
- Finite resolved dimensions, strict side clearance, and retained blind floor
  thickness exceeding ten linear tolerances are required. No automatic repair
  or shape approximation is applied to make an edit succeed.
- Unknown fields, operations, references and duplicate JSON fields are errors.
  Malformed/nonfinite JSON numbers and invalid tolerance policies are rejected.
- Documents are at most 64 KiB. WASM input is bounded and checked UTF-8.
- Operation IDs identify history nodes, not persistent face/edge names. Multiple
  bores, side-entry/history transforms, general Boolean nodes, constraints,
  STEP import/export and collaborative editing are not part of version 1.
- Display uses a 0.05 mm chord tolerance and the existing browser camera. A
  model that rebuilds but cannot produce a finite valid display transport has
  a rejected display report, not an invented successful mesh.

## Diagnostics and transport

Reports contain an authoritative `ok` boolean. Successful reports include
canonical operation document and derived display mesh. Failed reports include
`diagnostic`, display-only `candidate_segments`, and, when typed parsing succeeds,
`attempted_document`; they do not contain a replacement mesh.

Diagnostic fields are category, stable code, optional operation ID/field,
message, optional suggestion, and optional measured/required clearance in mm.
Known geometric preconditions have dedicated codes such as `side_clearance`
and `floor_thickness`. Unrepresentable clearance is `numerically_unresolved`.
Low-level geometry failures retain their actual reason and category; the API
does not pretend to classify every existing kernel failure numerically or
invent a recovery action. Parser failures may lack a geometric operation ID.

The WASM input ABI avoids unchecked pointers: call `hagane_workflow_begin()`,
then `hagane_workflow_push_byte(byte)` for each UTF-8 byte, and finally
`hagane_workflow_finish()`. Invalid bytes or excess length latch an input error
until the next begin. Read the existing output pointer/length afterward.
A zero finish status means the report was processed; it does **not** mean the
model succeeded. Always inspect `ok`. Output transport remains valid until the
next kernel output call and must be copied before further calls.

## Verified value and remaining work

Native tests independently check box/through/blind volumes, document round trips,
IDs/references, tolerance policy, unknown schema/units/fields, malformed documents,
clearance diagnostics, rejected-mesh absence and correction. WASM checks repeat
native geometry parity and analytic volumes across dimensions, bounded/invalid
UTF-8 input and recovery. Browser checks edit dimensions, reject/correct a bore,
retain the prior model, download/import JSON, switch modes and orbit the result.
The images above capture that renderer. CI executes these existing suites.

This delivers the first scoped milestone in [product direction](product-direction.md).
It does not establish superiority over existing CAD products. User task success,
rebuild performance and comparative measurements remain future validation work.
Incremental dependency rebuilding, broader operation histories and more precise
low-level numerical diagnostics remain planned.

Serialization uses Serde/serde_json in pure Rust; geometric modeling remains
Hagane's independent implementation. Dependency versions, sources and licenses
are in [references](references.md), with bundled
[license notices](workflow-dependency-notices.txt). Original code is MIT OR
Apache-2.0; no OCCT source was used.
