import assert from 'node:assert/strict';
import {meshDisplayScale, cameraProjection} from '../web/view-camera.mjs';

const raw = [-2, 1, 0, 2, -1, 0, 0, 0, 3];
const normals = Object.freeze([0, 0, 1, 0, 0, 1, 0, 0, 1]);
let reference;
for (const magnitude of [1e-150, 1, 1e150]) {
  const source = Object.freeze(raw.map(x => x * magnitude));
  const before = [...source];
  const result = meshDisplayScale(source, normals);
  assert.ok(Number.isFinite(result.scale) && result.scale > 0);
  assert.ok(Number.isFinite(result.radius) && result.radius > 0);
  assert.notEqual(result.positions, source);
  assert.equal(result.scale, 3 * magnitude);
  for (let i = 0; i < result.positions.length; i += 3) assert.ok(Math.hypot(...result.positions.slice(i, i + 3)) <= result.radius);
  assert.deepEqual(source, before);
  assert.deepEqual(normals, [0, 0, 1, 0, 0, 1, 0, 0, 1]);
  assert.equal(result.positions.length, source.length);
  assert.ok(result.positions.every(Number.isFinite));
  for (let i = 0; i < source.length; i++) {
    assert.ok(Math.abs(result.positions[i] * result.scale - source[i]) <= 4 * Number.EPSILON * Math.max(magnitude, Math.abs(source[i])));
  }
  const gpu = new Float32Array(result.positions);
  assert.ok([...gpu].every(Number.isFinite));
  for (let i = 0; i < raw.length; i++) if (raw[i] !== 0) assert.notEqual(gpu[i], 0);
  if (reference) for (let i = 0; i < gpu.length; i++) assert.equal(gpu[i], reference[i]);
  reference = gpu;
}
const empty = Object.freeze([]);
const cleared = meshDisplayScale(empty, empty);
assert.deepEqual(cleared, {scale: 1, radius: 1, positions: []});
assert.notEqual(cleared.positions, empty);
assert.deepEqual(empty, []);
for (const [positions, ns] of [
  [Array(9).fill(0), normals], [raw.map((x, i) => i === 4 ? NaN : x), normals],
  [raw.map((x, i) => i === 4 ? Infinity : x), normals],
  [raw, normals.map((x, i) => i === 4 ? NaN : x)],
  [raw, normals.map((x, i) => i === 4 ? Infinity : x)],
  [raw, normals.map((x, i) => i === 4 ? 1e150 : x)],
  [[1], [1]], [[1, 2], [0, 1]], [[0, 0, 0], [0, 0, 1]],
  [[1, 2, 3], []], [[1, NaN, 3], [0, 0, 1]], [[1, Infinity, 3], [0, 0, 1]],
  [[1, 2, 3], [0, NaN, 1]], [[1, 2, 3], [0, Infinity, 1]],
]) assert.throws(() => meshDisplayScale(positions, ns));

function rotate([x, y, z], yaw, pitch) {
  const a = x * Math.cos(yaw) + z * Math.sin(yaw);
  const b = -x * Math.sin(yaw) + z * Math.cos(yaw);
  return [a, y * Math.cos(pitch) - b * Math.sin(pitch), y * Math.sin(pitch) + b * Math.cos(pitch)];
}
let projected = 0;
for (const radius of [0.2, 1, Math.sqrt(3)]) for (const aspect of [0.25, 1, 4]) {
  const reset = cameraProjection(radius, aspect, 1);
  assert.equal(reset.zoom, 1);
  for (const requested of [1e-100, 0.5, 1, 2, 1e100]) {
    const c = cameraProjection(radius, aspect, requested);
    for (const key of ['distance', 'near', 'far', 'zoom', 'minZoom', 'maxZoom']) assert.ok(Number.isFinite(c[key]), key);
    assert.ok([...new Float32Array([c.distance, c.near, c.far])].every(x => Number.isFinite(x) && x > 0));
    assert.ok(c.minZoom > 0 && c.minZoom <= c.zoom && c.zoom <= c.maxZoom && c.maxZoom <= 8);
    assert.ok(c.distance >= 1.1 * radius * (1 - 8 * Number.EPSILON));
    assert.ok(c.near > 0 && c.near < c.distance - radius);
    assert.ok(c.far > c.distance + radius);
    for (const yaw of [0, 0.73, 2.4]) for (const pitch of [-1.1, 0, 0.62]) {
      for (let latitude = 0; latitude <= 24; latitude++) for (let longitude = 0; longitude < 48; longitude++) {
        const theta = Math.PI * latitude / 24, phi = 2 * Math.PI * longitude / 48;
        const p = rotate([radius * Math.sin(theta) * Math.cos(phi), radius * Math.cos(theta), radius * Math.sin(theta) * Math.sin(phi)], yaw, pitch);
        const depth = c.distance - p[2];
        assert.ok(depth > c.near && depth < c.far);
        if (c.zoom >= 1) {
          const screenX = p[0] / (depth * Math.tan(0.65 / 2) * aspect);
          const screenY = p[1] / (depth * Math.tan(0.65 / 2));
          assert.ok(Math.abs(screenX) < 1 && Math.abs(screenY) < 1, `${radius}/${aspect}/${c.zoom}: ${screenX},${screenY}`);
        }
        projected++;
      }
    }
  }
}
for (const [radius, aspect, zoom] of [[0,1,1],[-1,1,1],[NaN,1,1],[Infinity,1,1],[1,0,1],[1,-1,1],[1,NaN,1],[1,Infinity,1],[1,1,NaN],[1,1,Infinity]]) {
  assert.throws(() => cameraProjection(radius, aspect, zoom));
}
console.log(`View camera: extreme-scale normalization, source immutability, malformed inputs, and ${projected} independently projected sphere points passed.`);
