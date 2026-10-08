# Scoped planar face subdivision

![Actual WASM demo of an arc-cap split and refined cylinder walls](arc-face-split.png)

`split_planar_face` subdivides one planar B-rep face into two faces while
preserving a closed solid. Straight boundary cuts update adjacent planar faces.
Bounded arc cuts additionally subdivide the neighboring cylinder wall and its
opposite rim. The result refines topology while preserving geometry and volume;
it is not a solid cut or mesh Boolean.

```rust
use hagane::{Point3, Vec3, GeometryTolerance, rounded_rectangle_profile,
    extrude_arc_line, split_planar_face};
let tol = GeometryTolerance::default();
let profile = rounded_rectangle_profile(
    Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, tol.absolute(),
)?;
let solid = extrude_arc_line(&profile, 2.0, tol.absolute())?;
let split = split_planar_face(&solid, 0,
    Point3::new(0.0, 4.5, 0.0), Vec3::new(1.0, 0.0, 0.0), tol)?;
split.solid.validate(tol.absolute())?;
assert_eq!(split.solid.shell.faces.len(), 13);
```

Run `cargo run --locked --example arc_face_split` for this example or
`cargo run --locked --example part -- 8` for the browser fixture. Select
**Split curved cap** in the browser and enable **Show tessellation**. The cut
crosses two circular corner rims of an 80×60×24 mm rounded solid with fixed
24 mm corner radius. Its offset slider ranges from 8 to 24 mm. It has thirteen
faces and thirty-one edges, with unchanged volume
`[4800 - (4-pi)*576] * 24`. The same Rust operation executes in WASM.

The earlier straight cap with a fixed bore remains preset 7, with its
[actual screenshot](face-split.png). Run `cargo run --locked --example face_split`
for a simple box subdivision.

## Supported domain

- A validated planar face with a simple line/arc outer ring. Concavity works
  when the line produces exactly one interior interval and two proper crossings
  on **distinct** outer edges.
- Holes may contain lines, bounded arcs or full circles. The cut must not cross
  or touch them. Analytic containment assigns each unchanged hole wire to one
  child; no polygonized boundary is used for ownership.
- Straight crossed edges require planar neighbors and affine coedge pcurves.
  Planar neighbors can themselves contain circular or bounded arc trims.
- Crossed bounded arcs require one framed-cylinder neighbor with its existing
  four-coedge rectangular UV trim. Both bottom/top rims must be bounded arcs
  with angular ranges identical to the cylinder span. This operation refines
  the wall into two rectangles; it does not introduce arbitrary cylinder trims.
- Both pieces of each split boundary edge, including opposite rims, must have
  resolvable chord/length beyond ten linear tolerances. Full-circle outer edges
  must first be represented by bounded arcs; periodic single-circle refinement
  is not implemented.
- Vertex passage, tangencies, boundary overlap, multiple intervals, two hits on
  the same original edge, hole crossings and unresolved geometry return errors.
- Repeated cuts, reversed direction, top/bottom caps, inward notch walls, checked
  rigid placement and small dimensions work within these conditions. The line
  must satisfy the [planar clipping contract](face-intersections.md).

## Shared topology and parameters

The operation computes analytic trim clipping and works on a clone, so failures
leave the input unchanged. Each straight boundary event creates one vertex and
two subedges. The original edge index retains the first half; the second is
appended. Every coedge use is updated, with reverse uses reversing subedge order.
Affine pcurves are reparameterized to each new edge's normalized [0,1] interval.

For an arc event at angular parameter t, both cylinder rims split at that same
angle. The first subarc keeps its frame and range [0,t]; the second rotates its
frame's radial axes by t and restarts its angular parameter at zero. Planar arc
pcurves shift their start angles consistently. No circle is replaced by chords.

A new axial generator joins the bottom/top split vertices. The cylinder wall
becomes two faces sharing that generator with opposite signed uses. Each wall
retains a four-coedge rectangle starting at u=v=0, with spans t and span-t.
The second wall's frame rotates by t. The original wall orientation is retained,
including inward walls. The opposite planar cap receives matching rim subedges,
even when that cap is not the face selected for subdivision.

The selected outer wire partitions into two paths between cut vertices. A new
straight cut edge closes both paths with opposite coedge directions. Child faces
retain the parent's plane and orientation. Hole ownership uses analytic line/arc
classification of the child trims. Closure, shared uses, vertex links, winding,
pcurves and positive volume are validated before return; face-integrated volume
must agree with the original within 1e-10 relative error.

`PlanarFaceSplit` returns the new `solid`, both selected-face children in `faces`,
the `cut_edge` and `cut_vertices`. The first child replaces the selected face;
the second is appended after any newly refined cylinder faces. Original face
indices remain stable, though a refined cylinder face now represents its first
angular portion. For a straight event, V/E increase by 1/1. For an arc event,
V/E/F increase by 2/3/1. The selected face's chord adds another E/F increase of
1/1, preserving the Euler characteristic.

Planar B-rep trim validation permits forward straight subdivisions, including
mixed trims, while profile constructors still reject redundant corners.
Backtracking, short edges, self-intersection and near contacts remain invalid.
Display triangulation restores subdivision vertices. Arc cap and refined wall
sampling use the same subarc counts, preserving mesh seams and the circular
sagitta bound. Numerical checks remain f64 tolerance checks, not certified
circular or 3D predicates.

## Evidence and remaining work

Native tests cover box/rounded/disk/notch parts, polygon and curved holes,
analytic volume, top/bottom and repeated/reversed cuts, inward cylinder walls,
rigid placement, microscopic dimensions, shared pcurves and closed oriented
meshes. Native/WASM fixture parity and browser offset controls exercise nine
solid presets. Vertex, tangent, unresolved and same-edge cuts are rejected.

Cuts through holes, multiple intervals, periodic full-circle subdivision,
arbitrary curved-face trims, contact graphs, sewing and general Boolean
operations remain future work. No OCCT source or additional dependency is used.
