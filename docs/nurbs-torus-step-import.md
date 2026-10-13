# Bounded exact torus STEP import

`import_step_nurbs_torus_cardinal_mm(text, policy)` imports one regular ring torus
from its actual STEP geometry and shared topology. It returns `NurbsTorusSolid`,
with the same checked volume, mass, point/segment/line/ray, horizontal cut,
conforming display and STEP export APIs as a constructed torus.

```rust
use hagane::{import_step_nurbs_torus_cardinal_mm, GeometryTolerance};

fn inspect(text: &str) -> hagane::Result<()> {
    let policy = GeometryTolerance::default();
    let body = import_step_nurbs_torus_cardinal_mm(text, policy)?;
    body.validate(policy)?;
    println!("volume: {}", body.volume(policy)?);
    let mesh = body.tessellate(0.1, policy)?;
    let step = body.export_step_mm(policy)?;
    println!("{} triangles, {} STEP bytes", mesh.triangles.len(), step.len());
    Ok(())
}
```

## Supported representation

The input must use millimetres, one manifold closed shell, sixteen degree-(2,2)
3-by-3 rational tensor patches, sixteen vertices and thirty-two shared quadratic
NURBS edges. Each face has one four-coedge outer loop and all sixty-four affine
surface parameter attachments must have exact supported serialized preimages.
The original clamped `[0,0,0,1,1,1]` domains and quarter-circle weights are retained.
The frame is right-handed and aligned to signed coordinate axes, with a resolved
translation. Ring and tube clearance and world-coordinate arithmetic must satisfy
the supplied original full-body tolerance policy.

The first encoded shell patch anchors the major parameter phase and must encode
the canonical outer-equator-to-upper-minor quarter. Different valid major phase
anchors can return equivalent rotated cardinal frames and different canonical
index ordering. Record order and STEP identifiers may change; remaining face
order and cyclic loop starting positions are mapped bijectively. This is a
representation recognizer, not a general geometric-equivalence or shape-repair
importer.

## Geometry retention and inferred parameters

The reader decodes actual control points, weights, knots, curves, surface links,
edge endpoints, coedge orientation and raw affine UV records. The recognizer
infers a cardinal frame from the first control net and tests a bounded set of
radius candidates: the arithmetic seed first, followed by up to eight neighboring
floats on either side of each radius. Every accepted candidate must reproduce
**all** actual coefficients and the raw affine serialization exactly. Every
vertex, edge and face is reindexed through a complete bijection; actual imported
data are retained. The final typed B-rep certificate checks the entire body.
There is no tolerance fitting, near-match snapping or replacement with a newly
built candidate body.

STEP contains the represented geometry, not the original constructor's input
bits. Translation rounding can make several nominal radius values produce the
same complete encoded control nets. The importer deterministically selects an
exact represented preimage; inferred analytic radii or their metrics need not
recover lost nominal bits. Actual NURBS coefficients and topology remain
unchanged. Missing exact candidates or insufficient precision are explicit errors.

Arbitrary non-cardinal frames, unit conversion, analytic `TOROIDAL_SURFACE`
proxies, altered knot/weight bases, other parameterizations, imported horizontal
children, assemblies and general NURBS/torus STEP import remain unsupported.
Unrepresented or unused geometry, missing/inconsistent parameter attachments,
incompatible orientation, malformed data and unresolved dimensions are refused.
The existing frustum and graph importers retain their previous admission domains.
The parser and file example retain the existing 1 MiB input limit and bounded
record/work limits.

## Native and browser workflow

Run `cargo run --example nurbs_torus_step_import` for an actual generated cardinal
STEP round trip. `cargo run --example nurbs_torus_step_import -- part.step 0.1 1e-6`
reads a file; use `-` for stdin. Optional arguments are display chord error and
linear tolerance in millimetres. The output is the same actual torus report used
by the web viewer.

`NurbsTorusDemoSession::import_step_cardinal(text, chord, policy)` parses and
serializes the actual body before committing. Failed input, unsupported geometry,
invalid policy or display failure preserves the accepted source, cut children,
queries and exports. Successful import replaces the source and invalidates the
old child pair. The separate WASM byte transport is
`hagane_nurbs_torus_step_import_begin/push_byte/finish(chord, linear)`; invalid
bytes and malformed UTF-8 latch errors independently of all model/query buffers.
Its 2 MiB transport budget does not expand the core parser's 1 MiB admission limit.

The [torus demo](../web/nurbs-torus.html) accepts STEP text and files. It displays
and exports the actual imported body, then permits its read-only probes and local
horizontal cuts. Imported cardinal frames cannot all be represented by the
parametric model's single Y-angle field, so model editing is disabled in import
mode. Restore the privately remembered parametric model to edit those fields.
Failed import keeps the previous camera, geometry, source/child results and STEP.

Original code is MIT OR Apache-2.0. No OCCT implementation, geometry library or
new dependency was used; see [references](references.md) for provenance.
