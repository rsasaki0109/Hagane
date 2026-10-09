# Explainable editable stock-and-bore workflow

![Actual editable operation-history demo](workflow.png)

Open `web/workflow.html` after building the WASM module, or choose **Edit model**
from the main demo. Change width, length, height, tool center, radius and blind
depth. The same pure Rust kernel rebuilds and validates the exact B-rep. Switch
between through and top-entry flat-bottom blind bores. **Add bore** appends a
new machining operation; **Edit bore** selects a history node. Editing an earlier
hole preserves the later cuts. **Remove selected bore** removes that node and
relinks the remaining chain. The Stock only option removes the selected bore.

![Actual mixed blind/through operation history](workflow-multiple.png)

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
- History has one centered axis-aligned `box` or world-XY `extrusion` operation
  followed by at most
  256 `bore` nodes. Each input references the immediately preceding operation
  ID (the stock operation for the first bore); IDs are unique and
  contain 1–64 ASCII letters, digits, hyphens or underscores.
- Box size is `[width, length, height]`. Bore center is world XY in mm.
  `mode: "through"` has no depth (null is treated as absent), while `"blind"`
  requires finite positive depth entering the top/+Z cap. Through tools are
  generated with strict cap overhang, not inferred from blind depth.
- Finite resolved dimensions, strict side clearance, and retained blind floor
  thickness exceeding ten linear tolerances are required. All pairs of circular
  XY footprints must have separation greater than ten linear tolerances,
  including mixed blind/through pairs. Overlap, nesting and contact are rejected
  even if blind floors differ. No automatic repair
  or shape approximation is applied to make an edit succeed.
- Unknown fields, operations, references and duplicate JSON fields are errors.
  Malformed/nonfinite JSON numbers and invalid tolerance policies are rejected.
- Documents are at most 64 KiB. WASM input is bounded and checked UTF-8.
- Operation IDs identify history nodes, not persistent face/edge names. Side-entry/history transforms, general Boolean nodes, constraints,
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
`floor_thickness` and `bore_clearance`. The last code identifies the failed
operation and names the earlier conflicting operation, reporting their XY gap. Unrepresentable clearance is `numerically_unresolved`.
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
Mixed-history checks add a hole, edit an earlier depth without losing the later
cut, reject/correct pair contact, reload all nodes and remove/relink a node.
The images above capture that renderer. CI executes these existing suites.

This delivers the first scoped milestone in [product direction](product-direction.md).
It does not establish superiority over existing CAD products. User task success,
rebuild performance and comparative measurements remain future validation work.
The browser now uses an incremental `WorkflowSession` for the supported linear
history. Other operation types, persistent topology names and more precise
low-level numerical diagnostics remain planned.

Serialization uses Serde/serde_json in pure Rust; geometric modeling remains
Hagane's independent implementation. Dependency versions, sources and licenses
are in [references](references.md), with bundled
[license notices](workflow-dependency-notices.txt). Original code is MIT OR
Apache-2.0; no OCCT source was used.


## Mixed-bore native API

`subtract_box_bores(BoxSpec, &[BoxBore], Tolerance)` constructs the exact result
from the stock box and all independent tools. The
[mixed history example](workflow-multiple-example.json) is ready to import in the
browser or pass to `cargo run --example workflow -- <file>`. Each `BoxBore` stores XY center,
radius and optional depth (`None` means through). It supports up to 256 bores
and enforces the same ten-tolerance side/pair/floor margins as the workflow.
Existing single-mode APIs retain their original contracts. All result edges,
cap hole wires, cylindrical walls and blind floors use shared exact geometry
and pcurves; mesh Boolean operations are not used.


## Incremental B-rep evaluation

![Actual browser edit reusing two operations and rebuilding one](workflow-incremental.png)

`WorkflowSession` retains a validated exact solid after each accepted operation.
An edit compares the new history with the last accepted document and reuses the
longest unchanged prefix. Only the changed node and following nodes append new
exact B-rep geometry. Stock, units, schema or any tolerance-policy change prevents
prefix reuse (unsupported units/schema still fail). IDs and input references
participate in the comparison; renamed nodes are rebuilt conservatively.
Identical reloads and removing a suffix reuse the existing solid directly.
Removing an interior node relinks the chain and rebuilds the remaining suffix.

All parameter, reference, containment and pair-separation checks still run on
each request. Invalid inputs and failed geometry construction leave the accepted
snapshots intact. `evaluate_json` also stages cache changes until display
sampling/JSON succeeds, so a display-budget failure cannot replace the cache.
Returned geometry uses `Arc<Solid>`; mutating a caller-owned copy cannot corrupt
the session snapshots. `reset()` releases the session's snapshots.

```rust
use hagane::{WorkflowDocument, WorkflowSession};
let document: WorkflowDocument = serde_json::from_str(
    include_str!("../docs/workflow-multiple-example.json")
).unwrap();
let mut session = WorkflowSession::new();
let first = session.rebuild(&document).unwrap();
assert_eq!(first.stats.rebuilt_operations, 3);
let same = session.rebuild(&document).unwrap();
assert_eq!(same.stats.rebuilt_operations, 0);
```

Run `cargo run --example workflow_session` for three sequential reports (initial
build, identical reload, second-hole edit). Alternatively pass several JSON file
paths to evaluate them in order in one session. Successful incremental JSON
reports add `rebuild` with `reused_operations`, `rebuilt_operations`, and
`rebuilt_operation_ids`. These count actual stock/tool geometry evaluations;
they are not timings, display-cache counts or performance-comparison results.
Failed reports omit `rebuild` and replacement mesh. Browser statistics distinguish
an accepted rebuild from a rejected edit retaining the previous cache.

The stateless `WorkflowDocument::rebuild`, `evaluate_workflow_json` and WASM
`hagane_workflow_finish` preserve their existing API/report behavior. Incremental
WASM clients use the same bounded begin/push-byte input, then
`hagane_workflow_finish_incremental`; `hagane_workflow_reset_session` drops cached
history. One session belongs to each WASM instance. Documents contain no cached
geometry and still rebuild independently in native code or another browser.

This is geometry-construction reuse for a scoped linear history, not a general
constraint or dependency-graph solver. Complete input validation, topology
validation on newly built prefixes and final display tessellation are still
performed; there is no incremental mesh cache or performance superiority claim.
The session holds at most 257 prefix snapshots, with cumulative topology storage
quadratic in the number of holes. It has no persistence across page reloads or
cache of older edit branches. Browser Undo/Redo stores documents separately and
uses the session to reconstruct them. Native/WASM tests compare incremental results to fresh exact
rebuilds, exercise failure recovery and reset, and verify the rebuilt IDs. Browser
tests check the displayed counts, earlier/later edits, policy/stock invalidation
and cached suffix removal.


## Undo and redo browser edits

![Actual browser Undo/Redo with an editable mixed-bore part](workflow-undo.png)

**Undo** and **Redo** restore accepted operation documents, including dimensions,
operation IDs, references and the selected bore. Dimension changes, adding or
removing a bore, and successful JSON imports all participate. Each accepted
input event is one edit; identical model reloads do not add duplicate entries.
The editor keeps up to 64 previous accepted edits in memory. Old entries are
removed when that limit is exceeded.

A rejected geometry or document edit is never added to accepted history.
While a rejected edit is visible, Undo first discards that edit and restores the
current accepted model without stepping back another accepted edit. A second
Undo steps through accepted history. Redo is disabled until the rejected edit
is discarded or corrected. A failed edit preserves an existing redo branch;
a new successful changed edit after Undo clears that branch.

Restoration sends the saved document through the same pure Rust/WASM incremental
session, validates it and regenerates its display mesh. The undo cursor moves
only after successful restoration. It does not restore a mesh as a CAD model,
skip supported-domain checks, or claim cached old geometry is available.
Rebuild statistics describe this restoration's actual geometry work.

With focus outside text, numeric or selection controls, **Ctrl/Cmd+Z** undoes
and **Ctrl/Cmd+Shift+Z** or **Ctrl/Cmd+Y** redoes. Input controls retain their
normal text-editing shortcuts. Buttons are available regardless of focus.
Unloaded text in the JSON editor is not a model edit; use Load document to apply
it. Undo/Redo does not restore camera position, unsubmitted JSON text or rejected
input text. Download still saves only the current validated modeling document,
not undo stacks. Reloading the page starts a new edit history while restoring the locally saved
current document as described below.

Browser regression checks exercise exact restored documents and volumes, bore
selection, add/remove/import recovery, rejected edits without lost redo,
branch replacement, keyboard focus and the 64-edit bound. Existing native/WASM
session parity and geometry validation remain the modeling path.


## Local autosave and recovery

![Actual validated restoration from local browser storage](workflow-autosave.png)

Every accepted edit saves the current operation document and selected bore to
this browser's local storage. Undo/Redo and selection changes update that saved
current model too. On page reload, Hagane reconstructs it through the same
Rust/WASM validation and B-rep tessellation; it does not trust or save a display
mesh. The restored model starts a fresh Undo/Redo history and geometry session.
Rejected edits never replace the saved accepted document.

A local wrapper uses `storage_version: 1`, `document` and `selected` under
`hagane.workflow.autosave.v1`. Unknown wrapper versions/fields, invalid selection,
malformed/oversized data and unsupported or invalid model documents are rejected.
The wrapper is bounded to 65 KiB, and the kernel document retains its separate
64 KiB input limit. Invalid saved data remains intact while the default part is
shown with a recovery message. **Save this tab locally** explicitly replaces
that data with the current validated part. Correct/discard a rejected edit before
using that button; it cannot save a failed tool as a valid model.

If another tab writes or clears the saved model, this tab keeps its current
part and pauses automatic writes. Each write also checks the last saved value,
so a missed or delayed storage event cannot silently overwrite a newer value.
Use **Save this tab locally** to choose this tab's valid model deliberately.
This is conflict detection for local tabs, not collaborative editing or an
atomic cross-tab transaction; simultaneous read/write races remain possible.

Unavailable storage or quota errors display a save failure without rejecting
a successfully constructed model. Download editable model remains available
for a portable JSON copy. Local storage belongs to the browser profile and
site origin: another browser, port or origin has a separate saved model. This
is not cloud synchronization, a backup guarantee or persistent Undo/Redo.
Browser checks cover reload/selection recovery, rejected-edit preservation,
malformed/invalid saved data, explicit recovery, competing tabs, quota failures
and storage denial without breaking modeling.


## Editable polygon extrusion and subsequent holes

![Actual extruded polygon part with editable mixed bores](workflow-extrusion.png)

Select **Polygon extrusion** under Stock operation to replace the root box with
an eight-corner polygon using its current width/length. Select Box to explicitly
replace the polygon (including its profile openings) with a centered box of its
displayed bounding width/length.
Switching stock type changes the part geometry; it is an accepted undoable edit.
The root operation ID is preserved and bores still reference the preceding node.

For an extrusion, **XY polygon vertices (JSON)** and **Apply profile** edit the
actual boundary. Height remains editable; width/length display the profile's
bounds and are disabled. Apply profile and full-document Load pass input to the
Rust parser and kernel validation. There is no triangulated mesh used as stock.
The [extrusion example](workflow-extrusion-example.json) is ready to load in the
browser or with `cargo run --example workflow -- <file>`.

Version 1 now accepts this additional root operation:

```json
{"kind":"extrusion","id":"extrusion-1",
 "outer":[[-40,-20],[-30,-30],[30,-30],[40,-20],
          [40,20],[30,30],[-30,30],[-40,20]],
 "height":24}
```

`outer` has 3–256 finite world-XY vertices with an implicit closing edge, in
either winding. The profile may be concave but must be simple, resolved and have
no redundant corners or self-intersections. Polygon profile openings are supported as described below; curved profiles
and arbitrary-plane extrusion frames are outside this document scope. Skew
directions for polygon stock are supported as described below. Height must be positive and exceed ten linear tolerances; stock spans
Z = −height/2 to +height/2. Negative/zero heights and unknown fields are rejected.
Existing box documents keep their prior behavior and schema version.

Up to 256 disjoint top-entry blind/through bores can follow the extrusion. Each
circular footprint must lie entirely inside the polygon, with clearance greater
than ten linear tolerances from every straight boundary segment. Center-inside
classification plus nearest-segment distance checks actual concave boundaries,
not only a bounding box. Outside centers have a negative signed margin; boundary
contact, overlap, nesting and unresolved floors are rejected. `side_clearance`
identifies the failed bore and reports the measured/required margin.
`profile_rejected` identifies an invalid or unsupported typed stock profile.
Malformed JSON/coordinate types produce the existing document-parser diagnostic.

Native `subtract_polygon_prism_bores(outer, height, &[BoxBore], tolerance)` exposes
the same scoped exact operation; `BoxBore` is the shared existing center/radius/
optional-depth tool specification. The stock is created with `extrude_polygon`.
Plane cap wires share exact circle edges with inward cylindrical walls and
blind floors. Display tessellation follows B-rep validation. No OCCT source or
additional dependency is used.

Changing height or the profile invalidates the stock node and all following
cuts. A later bore edit still reuses unchanged earlier B-rep snapshots. Browser
Undo/Redo, JSON downloads/imports and local autosave include the extrusion root.
Tests cover analytic area×height minus bore volumes, both windings, tiny/large
and translated XY profiles, material under blind floors, profile/bore contact
and self-intersection rejection, incremental results equal to fresh builds,
native/WASM parity and the real browser workflow. Mesh-volume comparison uses
the circular chord-error bound; the display mesh is not treated as exact volume.


## Polygon profile openings

![Actual extruded polygon openings with blind and through bores](workflow-extrusion-openings.png)

An extrusion root may now include optional `holes`, an array of simple polygon
loops in the same world-XY coordinate system as `outer`. These are through
openings in the stock profile, distinct from later bore operations. Missing or
empty `holes` preserves all existing version-1 extrusion documents; empty holes
are omitted from canonical exports. Null, malformed coordinates and unknown
fields are errors. Load the [opening example](workflow-extrusion-openings-example.json)
in the browser or native workflow example.

```json
"holes": [[[-34,-8],[-22,-8],[-22,8],[-34,8]]]
```

Use **Polygon openings (JSON array of loops)** and **Apply profile** to edit
these loops alongside the outer boundary. Each loop has at least three finite,
resolved corners and an implicit closing edge, in either winding. The stock
supports up to 64 profile openings and 256 total corners across outer/inner
loops. Loops must be strictly inside the outer boundary, mutually disjoint,
non-nested and separated by more than ten linear tolerances. Self-intersections,
duplicate/redundant corners, overlap and contact are rejected as
`profile_rejected` on the stock operation. Changing openings invalidates the
stock node and its following cuts; height and bore edits preserve openings.

A circular bore must lie in stock material and remain more than ten linear
tolerances from every outer and inner boundary. A tool inside an existing
opening, touching or crossing an opening, or enclosing an opening is unsupported.
`profile_hole_clearance` identifies the bore, gives the signed margin and required
margin, and names the conflicting profile opening by zero-based index. Failed
edits return no replacement mesh, retain the accepted cache/model and do not
replace local autosave. Correcting or Undoing them restores the prior document.

Native `subtract_polygon_region_prism_bores(outer, holes, height, bores, tolerance)`
uses the existing exact polygon-region extrusion and circular-cut construction.
The original `subtract_polygon_prism_bores` remains the empty-profile-hole
convenience API. Polygon opening side walls, cap boundaries, circular bore walls
and blind floors all use shared exact edges and surface pcurves. The displayed
mesh is generated afterward; no mesh Boolean or polygon-to-circle replacement
is used. JSON save/load, autosave and Undo/Redo preserve the actual opening loops.

Tests verify independent area-minus-openings volumes minus mixed bore volumes,
closure/orientation, material and void probes, both windings, small/large scales,
mesh-volume chord bounds, profile/opening/tool rejection, incremental cache
recovery, native/WASM equality and the browser edit/download/reload workflow.
Rounded/curved opening loops, general intersecting cuts and arbitrary-plane
extrusion nodes remain later work. Skew polygon stock is supported below. Mathematical provenance remains the planar
containment/boundary-distance references recorded in [references](references.md);
no new dependency or OCCT source was introduced.


## Skew extrusion direction in history

![Actual editable skew extrusion with a polygon opening](workflow-skew-extrusion.png)

Polygon roots now accept optional `offset: [dx, dy]` in mm. The lower cap uses
the original world-XY profile at Z = −height/2; the upper cap is translated by
`[dx, dy, height]`. This specifies a skew translation, not a rotation of the
profile plane. Height remains the positive Z span, not the slanted path length.
All profile openings sweep along the same vector. Finite offsets are required;
malformed arrays/nonfinite JSON are rejected. Omitted or zero offset retains the
old vertical operation; zero offsets are omitted from canonical exports.

The native workflow reuses the existing pure Rust `extrude_polygon` direction
API, preserving exact planar caps, side walls, shared line edges and surface
pcurves. Native results also support classification, bounds and B-rep-derived
mesh generation. Volume is profile material area × height regardless of the
XY offset. The [skew example](workflow-skew-extrusion-example.json) is ready for
native evaluation and browser import.

Edit **Top offset X/Y** in the polygon controls. Direction changes invalidate
the stock node; Undo/Redo, downloads, local autosave and reload preserve direction
and profile openings. Applying profile edits also preserves offsets. Switching
to Box explicitly replaces skew stock with a centered box and clears offsets.

**Bore nodes on skew stock are unsupported.** Any nonzero offset, including one
smaller than linear tolerance, returns `unsupported_skew_bore` at the bore ID;
it is not snapped to zero and an existing bore is not silently dropped. Remove
bore nodes to create skew stock, or set both offsets to zero to use the supported
vertical-stock bores. Failed combinations preserve the previous validated model,
accepted cache and local save. This does not claim vertical cylinder clipping
against moving/skew side walls or a general Boolean operation.

Tests cover exact volume, bounds, opening/material/boundary probes, closed
oriented topology, display volume, tiny/large geometry, native/WASM equality,
direction invalidation and failure cache recovery. Browser checks cover direction
editing, profile application, Undo/Redo, JSON download/reload and rejecting a
bore before returning to supported zero-offset cuts.
