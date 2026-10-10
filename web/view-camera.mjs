// Display-only similarity transform and perspective camera; CAD data is untouched.
export const VERTICAL_FOV = 0.65;

export function meshDisplayScale(positions, normals) {
  if (!positions || !normals || positions.length !== normals.length
      || positions.length % 9 !== 0) {
    throw Error('Invalid display mesh.');
  }
  // An empty Boolean intersection intentionally clears the existing display.
  if (positions.length === 0) return {scale: 1, radius: 1, positions: []};
  let scale = 0;
  for (let i = 0; i < positions.length; i++) {
    if (!Number.isFinite(positions[i]) || !Number.isFinite(normals[i])
        || !Number.isFinite(Math.fround(normals[i]))) {
      throw Error('Display mesh coordinates and normals must be finite.');
    }
    scale = Math.max(scale, Math.abs(positions[i]));
  }
  if (!(scale > 0)) throw Error('Display mesh has no resolved extent.');
  const normalized = Array.from(positions, value => value / scale);
  let radius = 0;
  for (let i = 0; i < normalized.length; i += 3) {
    radius = Math.max(radius, Math.hypot(normalized[i], normalized[i + 1], normalized[i + 2]));
  }
  return {scale, radius, positions: normalized};
}

// zoom is a distance ratio: values below one zoom in; values above one zoom out.
export function cameraProjection(radius, aspect, zoom = 1) {
  if (![radius, aspect, zoom].every(Number.isFinite) || radius <= 0 || aspect <= 0) {
    throw Error('Invalid camera bounds.');
  }
  const halfFov = Math.min(VERTICAL_FOV / 2,
    Math.atan(Math.tan(VERTICAL_FOV / 2) * aspect));
  const fitted = 1.1 * radius / Math.sin(halfFov);
  const minZoom = 1.1 * radius / fitted, maxZoom = 8;
  const clamped = Math.max(minZoom, Math.min(maxZoom, zoom));
  const distance = fitted * clamped, near = distance * 0.001, far = distance + 4 * radius;
  if (![distance, near, far].every(Number.isFinite) || !(near > 0 && near < far)) {
    throw Error('Camera bounds exceed display precision.');
  }
  return {distance, near, far, zoom: clamped, minZoom, maxZoom};
}
