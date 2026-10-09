# Product direction: measurable value beyond feature coverage

This is a development target, not a claim that Hagane currently outperforms
established CAD software. Exact geometry and robust B-rep operations remain
foundational. User value, rather than the number of special cases or presets,
now determines the priority of the next milestones.

## Initial users and problems

Start with developers embedding exact parametric modeling in web applications
and people repeatedly producing dimensional variants of mechanical parts.
They need to understand failed operations, change a model without restarting,
and reproduce or share the exact construction rather than only a display mesh.
Browser execution and pure Rust are enabling architecture; neither alone is
proof of better usability, accuracy, performance or reliability.

## Three outcomes to build and validate

1. **Explain failures and help resolve them.** Structured diagnostics identify
   the operation, relevant input/entity, reason, measured clearance and required
   tolerance when these are known. The viewer marks the candidate feature or
   geometry and explains a concrete permitted correction. Distinguish invalid,
   unsupported and numerically unresolved cases. Do not fabricate a remedy or
   apply silent repair. A rejected edit preserves the last valid model, clearly
   labeled as the previous result rather than success for the rejected input.
2. **Reproduce editable modeling intent.** A versioned operation document stores
   units, tolerance policy, input parameters and operation IDs. Loading it
   rebuilds and validates exact B-rep using the kernel. Native and WASM results
   must agree within documented numerical budgets. Operation identity must not
   be confused with persistent face/edge naming; stable topology references
   require their own later implementation and tests.
3. **Make precise editing accessible in the browser.** Users create a part,
   edit dimensions and see the validated result and diagnostics without a
   server-side geometry dependency. They can export/import the operation
   document and resume editing. Display meshes remain derived artifacts;
   saving a mesh is not saving exact geometry or an editable model.

## Next milestone: an explainable editable bore workflow

Deliver one integrated modeling workflow instead of another isolated preset:

- Create a box with explicit dimensions and units, then add a supported
  through or blind cylindrical bore through its documented primitive operation.
- Show the actual operation history and allow edits to box dimensions, tool
  center, radius and depth. Rebuild and validate the affected result.
- For side contact, excessive depth, nonfinite values and unsupported geometry,
  report the specific failed operation and actionable diagnostic. Highlight
  its candidate tool even when no resulting solid can be generated.
- Export and import a versioned JSON operation document. Reject unknown schema
  versions, units, operations and invalid parameters explicitly. Rebuilding
  through native and WASM yields equivalent topology, geometry and metrics.
- Keep the prior valid result visibly distinct during a failed edit. Correcting
  the input resumes normal construction without losing the user's history.
- Test real browser editing, failure/correction and document round trips, plus
  independent analytic volume, topology closure and native/WASM parity.

The first document format covers only this supported workflow. It does not
claim general Boolean operations, STEP compatibility, stable topology naming,
collaborative editing or a complete parametric CAD system.

## Evidence before superiority claims

Define reproducible tasks and published workloads: time to first editable part,
ability to diagnose and correct an invalid operation, exact-model round-trip
success, validated rebuild latency and peak memory. Record hardware, versions,
tolerance policies, supported domains and failures. Compare identical supported
geometry and approximation settings; report p50/p95 rebuild latency separately
from rendering. No speed, accuracy or reliability advantage is claimed without
measurements against the named reference implementation and version.

Collect user feedback on the editing and diagnostic workflow before broadening
it. Keep general intersections/Booleans, NURBS B-rep, fillets and STEP on the
long-term roadmap, prioritizing the prerequisites needed by actual workflows.
AI-assisted modeling can later consume explicit typed operations and diagnostic
results; it must not silently substitute approximate geometry or guess success.
