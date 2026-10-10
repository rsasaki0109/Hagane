# Scale-aware WebGL display

![Actual cut-and-bored B-rep with the default fitted camera](workflow-plain-continuation.png)

The shared viewer now fits the actual display mesh instead of using camera
distance 205 and zoom limits 120–350 in model units. A 10 mm cut child can
be inspected at a useful size; a large part can fit without the old
1000-unit far-plane clipping. Orbit, wheel/keyboard zoom and Reset operate
on the same display-only camera. CAD coordinates, units, metrics, topology,
pcurves and STEP remain unchanged.

## Display transform and camera

Received mesh positions are divided by their maximum absolute coordinate
before upload to Float32 GPU buffers. A circumsphere about the received
origin bounds these normalized positions. This is a similarity transform
for rendering; it does not edit the source mesh or choose a new CAD origin.
Callers' existing display centering is preserved. A far-translated part whose
caller deliberately retains the origin may still appear small.

Default/Reset distance is `1.1 * radius / sin(half_fov)`. The half-angle is
the smaller of vertical and horizontal view angles, so portrait viewports
also fit. Zoom is a dimensionless distance ratio preserved across accepted
geometry edits. Zoom-in stops outside the bounding sphere; zoom-out and
near/far clipping scale with the actual bounds. Resize recomputes the fit
from the current aspect ratio. Deliberate zoom-in may crop the model.

Markers and candidate segments use the same normalization. Their original
coordinates are retained and re-uploaded when an accepted mesh changes scale.
The viewer's decorative normal arrow has length 0.28 of the display sphere
radius; it represents direction, not a measured CAD edge. Some callers
construct their own fixed-unit tangent arrows as segments; those lengths
remain unchanged. Overlays outside finite Float32 range are declined without
changing the accepted body/camera; setters return false and the readonly
`canvas.haganeCamera.overlayVisible` reports false.

Positions, normals and display bounds are checked before accepted buffers or
camera state change. Finite normals that overflow Float32 reject. Empty
meshes intentionally clear the viewport for a real empty Boolean intersection.
Degenerate nonempty meshes and nonfinite data reject. This renderer validation
is not CAD shape validation or a general screen-space approximation guarantee.
Existing Rust tessellation error contracts apply in original model units.

`canvas.haganeCamera` exposes a readonly snapshot of normalized radius, scale,
distance, clipping planes, distance-ratio zoom, angles, viewport aspect, vertex
count and overlay visibility for integrations and diagnostics.

## Verification

```sh
node scripts/test-view-camera.mjs
```

Independent tests project 486000 sphere points across radii, viewport aspects,
yaw/pitch and zoom states. Default/zoom-out positions fit; all allowed zoom
states retain positive depth and safe near/far planes. Frozen input arrays at
scales 1e-150, 1 and 1e150 retain equivalent finite Float32 display positions
without source mutation. Malformed/nonfinite/collapsed meshes and excessive
normal magnitudes reject; empty intersection display remains supported.
These checks run in CI alongside the existing kernel/WASM/browser checks.

The complete browser suite passed with the shared viewer change. Actual Rust/WASM
cut-and-bored solids at dimension scales 1, 1e-4 and 100 retain the analytic
volume `(400 - 5*pi) * scale^3`, unchanged mesh data and byte-identical STEP
through orbit, wheel/keyboard zoom and Reset. Framebuffer bounds verify useful
unclipped default occupancy; Reset restores the original GPU pixels exactly.
Invalid CAD edits preserve the accepted mesh and camera.

Additional browser checks compare actual GPU pixels for retained raw markers
and segments after mesh rescaling, reject nonfinite/collapsed meshes and
Float32-overflowing normals without changing accepted pixels/camera, clear an
empty mesh, and decline an unrepresentable distant overlay without changing
the accepted body. The refreshed image above was captured from this running
implementation, without manual capture zoom or a substitute model.
