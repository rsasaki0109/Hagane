# Coaxial holes and enlargement in retained frusta

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

Checked adapters extend these operations to the retained typed families.
Plain `NurbsObliqueFrustumSolid` and `NurbsFrustumSlabSolid` provide drilling
and explicit partition methods returning the new bored plane-bounded type.
Bored oblique children and bored parallel slabs provide strict enlargement and
annular partition methods returning that same type. Canonical
`NurbsBoredFrustumSolid` enlargement retains the canonical bored type, with a
canonical bored annulus as the explicit removed body. Its
`partition_coaxial_bore_enlargement` returns
`NurbsBoredFrustumBoreEnlargement {kept, removed}`.

The adapters validate the complete old body and its source association before
admitting the actual retained B-rep. They do not recognize an arbitrary `Solid`
by face counts or replace it with a similar source recipe. The original outer
geometry and section bounds remain checking witnesses, with the same original
source tolerance and global minimum radius restrictions.

The schema-1 workflow's `coaxial_bore` can consume one plain canonical,
oblique, slab or plane-bounded component. The separate `enlarge_coaxial_bore`
operation takes `id`, `input` and `radius`, and accepts one supported bored
component from those typed families.
Multi-component inputs require explicit selection. The accepted component is
used directly through edits, queries, Undo/Redo and export; failed operations
preserve accepted state, cache and history.

The [editable retained-hole fixture](nurbs-frustum-workflow-retained-bore-example.json)
selects a skew-cut middle component, drills radius 3, then enlarges it to radius
4. Load the retained-hole preset in the frustum workflow web demo to inspect it.
The [parallel-hole fixture](nurbs-frustum-workflow-parallel-hole-example.json)
uses the existing strict parallel-cut operation, then drills and enlarges the
explicitly selected middle slab. The parallel-hole preset displays that actual
component with the same radius controls.

The existing parallel-cut restrictions and import APIs keep their previous
domains. These additive checked bore adapters do not add off-axis or blind
frustum bores, arbitrary shell recognition, machining of other shape families,
general curved Booleans or generic STEP import.

This original implementation uses positive homogeneous ruled patches, oriented
shared topology and checked common-local moment composition. No OCCT code or
new dependency is introduced; code remains MIT OR Apache-2.0.
