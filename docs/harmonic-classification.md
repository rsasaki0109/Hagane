# Solid classification with harmonic circular wall bands

![Actual WASM classification of an obliquely subdivided solid](harmonic-classification.png)

`classify_point_in_solid` supports the harmonic height bands produced by
[oblique plane boundary subdivision](oblique-boundary.md), including normal and
skew circular translations, holes, signed extrusion and rigid placement.
It classifies the actual validated closed B-rep. Splitting the exterior into
more faces does not create an interior cutting face or change its material.

## Boundary distance

A wall band has `lower(u) <= v <= upper(u)` with each bound
`a + b cos(u) + c sin(u)`. At each angle the exact wall is a straight segment
along the generator. On an angular interval, replacing both rim curves by their
chords gives a convex planar trapezoid. The two generator sides may have
different lengths; orthogonal plane projection and segment projection bound the
distance to this trapezoid.

For radius `R`, generator vector `g` per normal-height unit and interval width
`h`, the correspondence error between the analytic patch and trapezoid is at
most `K*h^2/8`, where

`K = R + |g| * max(hypot(lower.b, lower.c), hypot(upper.b, upper.c))`.

This follows from the second derivative bound for linear interpolation on each
rim and convex interpolation along generators. The Hausdorff-distance
inequality converts it to a lower bound on distance to the exact face. Points
on actual bounded generators provide upper bounds. Refinement determines
whether the point lies within the Euclidean tolerance band; it never uses the
display mesh. Checked binary64 roundoff and projection-conditioning margins
are included. These are checked numerical allowances, not interval arithmetic.
Unresolved bounds, ill-conditioning or insufficient coordinate precision return
errors. Refinement is capped at 16,384 visits and depth 48.

## Inside and outside

Analytic ray intersections are clipped to the two height graphs. Hits near an
angular boundary, harmonic rim or tangency make that ray unresolved rather than
being counted twice on shared edges. The harmonic residual is scaled by its
physical plane normal before comparison with the length budget. Two independent
resolved rays must agree; crossings must alternate entry and exit with the
stored face orientations. Inward hole walls therefore retain their meaning.
Topology and supported trim domains are checked before bounding-box shortcuts.

## Demo and verification

```sh
cargo run --locked --example curved_classification -- 6 26 -3 0
cargo run --locked --example curved_classification -- 6 6 -3 0
cargo run --locked --example curved_classification -- 6 --mesh
./scripts/build-web.sh
```

Serve `web/` using the README instructions and open `classification.html`.
Choose **Oblique sections and rounded hole**, the eighth classification model.
The unchanged volume is `64317.450789637... mm³`; the actual subdivided solid
has 34 faces and 80 edges. Material, hole, curved wall, cap, corner and exterior
probes use the same native/WASM Rust implementation. Tessellation displays the
shared harmonic boundaries; orbit and zoom work as before.

Native tests independently check material/hole membership before and after
subdivision, both signs, rotation, microscopic dimensions, all section edges,
caps and inward/outward normal offsets. Invalid topology and an unresolved
point exactly at the tolerance threshold are tested. WASM checks compare query
results and meshes with native fixtures; browser tests exercise all eight
models and their probes.

Arbitrary trim loops, geometric self-intersection detection, repeated harmonic
band subdivision, partial/rim plane cuts, capped solid partitions and general
curved Booleans remain unsupported. This extends classification, not solid
partitioning. Original code is MIT OR Apache-2.0; no dependencies or OCCT source
were used. Mathematical references are the linear-interpolation remainder from
Taylor's theorem, convex plane/segment projection and the Hausdorff-distance
triangle inequality, extending the references in
[skew classification](skew-classification.md).
