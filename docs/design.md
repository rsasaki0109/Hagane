# Kernel design and invariants

## Layers

`math` → `geometry` → `topology` → `operations` → `mesh`. The `wasm` module is
an ABI adapter to the same kernel, not a second implementation. WebGL receives
only display triangles and metadata. No system CAD library or C/C++ geometry
code is linked.

Primitive descriptors are the initial narrow-phase Boolean operands. The
operation computes plane/cylinder intersection circles at the box caps, adds
trim wires, and constructs the resulting oriented shell. Geometry/topology
have no dependency on the primitive descriptors: later intersection,
classification, splitting, and sewing algorithms can operate on the same
edge/coedge/face model. This primitive operation is not a generic CSG engine.

## Geometry and units

Coordinates and lengths are `f64`, in a consistent caller-selected unit.
`Tolerance::linear` is positive and finite. There is no automatic unit scaling.
Primitives reject dimensions ≤10 tolerances. Curves use normalized `[0,1]`
line parameters and `[0,2π]` circle parameters. Circles lie in XY; cylinder
parameters are `(angle, axial length)` with the angle in radians. Plane axes
are unit and orthogonal; their UV coordinates use length units.

`Transform` maps points with a right-handed orthonormal basis and translation.
It does not silently transform solids or promote an axis-aligned Boolean to a
rotated one. Low-level curve/surface evaluation is mathematical evaluation;
solid constructors and intersection functions provide checked entry points.

## Topological ownership

A solid owns indexed vertex and edge arenas and one connected shell. Edges
own exact curves and refer to endpoint vertex IDs. A closed circle has one
vertex used at both endpoints. Faces refer to edges through coedges; there
is no copy of a shared edge for each face.

Every coedge has a pcurve evaluated at the **same parameter as its 3D edge**.
`forward` controls traversal only. A plane pcurve is an exact affine line or
circle. The cylinder's bottom/top rings have affine angle/height pcurves.
The seam has two uses on the same face, at `u=0` and `u=2π`, sharing one 3D edge.
The complete cylinder wire closes in unwrapped UV space.

Wires follow the positive surface parameter orientation: outer CCW, holes CW.
`Face::orientation` is +1 or -1 and reverses its surface normal and all edge
uses together. Box bottom caps reverse the XY plane; a bore reverses the
cylindrical wall. Each edge has exactly two signed uses with opposite signs,
including the cylinder seam. The link of each vertex is a single cycle.

Planar cap holes are exact circle wires. A through-hole result has 10 vertices,
15 edges and 7 faces; two cap faces contain inner wires. Accounting for those
inner wires gives Euler characteristic 0 (genus one), rather than pretending
every multiply-connected face is a disk. Unused entities, open wires, invalid
indices, inconsistent endpoints/pcurves, invalid face bases, unsupported trim
domains, disconnected shells/vertex links, and nonpositive volume fail validation.

Pcurve agreement is checked at endpoints and intermediate parameters. Together
with the restricted analytic trim forms this is a practical structural check;
it is not a certificate for arbitrary newly introduced curve types. Extend
validation when extending geometry.

## Exact volume and approximate display

Volume is an analytic divergence-theorem integral over faces, not an operation
history formula and not a mesh volume. Planes contribute oriented area times
`(origin-reference) · normal / 3`. Full cylinders contribute oriented
`2π r² h / 3`; the translation terms integrate to zero. A nearby reference
point reduces cancellation from global translations. Exact circle loop areas
subtract the cap holes. This yields `box volume − π r² h` independently of
display settings.

Bounds include vertices and analytic circle extrema. The display mesher samples
shared curves with a radius/error-dependent segment count, triangulates planar
trim polygons with the Rust `earcutr` dependency, and evaluates cylindrical
patches directly. It does not create or subtract mesh solids.

For a circle chord of angle Δθ, sagitta is `r(1-cos(Δθ/2))`. The segment count
uses `Δθ ≤ 4 asin(sqrt(error/(2r)))`, with a minimum of 12 segments and a
maximum of 65,536. A cylindrical chord patch has the same radial bound; planar
surfaces have zero surface deviation, while circular trim boundaries use the
same sagitta bound. Face/edge provenance stays explicit; display normals are
radial on cylindrical faces and constant on planes. Closed-mesh tests weld
positions only for adjacency checks; rendering intentionally splits vertices.

## WASM boundary

The Rust `cdylib` exports `hagane_generate(radius, chord_error) -> status`,
`hagane_output_ptr()`, and `hagane_output_len()`. JSON bytes contain coordinates,
normals, analytic volume, and topology counts, or an explicit error. Status 0
means success, 1 means a kernel error. The buffer is valid until the next
`hagane_generate`; callers must copy/decode it before generating another part
and reacquire the memory view after memory growth. This is a single-instance,
synchronous demo ABI, not yet a versioned public serialization format.
