# Coaxial holes in retained plane-bounded frusta

`NurbsFrustumPlaneBoundedSolid::bore_coaxial(radius, policy)` adds one coaxial
through opening to actual retained material. The additive
`NurbsBoredFrustumPlaneBoundedSolid::enlarge_coaxial_bore(radius, policy)` removes
the annular material between the old and new radii. The convenience methods return the actual kept body.
`partition_coaxial_bore` returns `{kept, removed}`, with the removed body a
`NurbsFrustumPlaneBoundedSolid`; `partition_coaxial_bore_enlargement` returns
`{kept, removed}` with both bodies `NurbsBoredFrustumPlaneBoundedSolid`.
These explicit partitions expose an actual closed removed body. This is a typed
cylindrical difference, not a mesh subtraction or an export of the uncut source.

The outer plane-bounded geometry remains the actual retained outer body. New
inner rational walls meet the same lower and upper cap planes, with annular cap
wires and inward orientations. The removed cylinder or annular shell has its own
closed oriented B-rep. Both bodies are checked against the original canonical
source and retained section boundaries; general reboring is not inferred from
similar-looking faces.

The radius, remaining wall and removed annular width must be resolved under the
original finite source's absolute and relative tolerance policy. Conservative
admission uses the original source's minimum radius; a locally wider retained
section does not relax that bound. Contact, shrinking an existing hole,
insufficient positive clearance, nonfinite input or unresolved world arithmetic
return explicit errors. Enlargement requires a strictly larger radius.

The kept and removed bodies use the existing actual plane-bounded display,
checked local analytic mass and finite-face query APIs. Positive rational wall
weights determine generator sampling and face UV mapping. Optional mass
conditioning errors remain explicit while successful geometry, stable volume,
bounded display and STEP remain available. Reports never substitute a source
centroid or a zero tensor. STEP export writes each actual body; generic STEP
import of these retained shapes remains unsupported.

The schema-1 workflow's existing `coaxial_bore` can now consume one plain
`PlaneBounded` component. The separate `enlarge_coaxial_bore` operation takes
`id`, `input` and `radius`, and accepts one `BoredPlaneBounded` component only.
Multi-component inputs require explicit selection. The accepted component is
used directly through edits, queries, Undo/Redo and export; failed operations
preserve accepted state, cache and history.

The [editable retained-hole fixture](nurbs-frustum-workflow-retained-bore-example.json)
selects a skew-cut middle component, drills radius 3, then enlarges it to radius
4. Load the retained-hole preset in the frustum workflow web demo to inspect it.

Earlier canonical, single-cut oblique and parallel-slab APIs retain their
previous domains. This does not add off-axis or blind frustum bores, arbitrary
shell recognition, continued machining of other typed shapes, general curved
Booleans or generic STEP import.

This original implementation uses positive homogeneous ruled patches, oriented
shared topology and checked common-local moment composition. No OCCT code or
new dependency is introduced; code remains MIT OR Apache-2.0.
