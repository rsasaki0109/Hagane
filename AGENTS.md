# Hagane development instructions

Read [GOAL.md](GOAL.md), [README.md](README.md), and
[docs/roadmap.md](docs/roadmap.md) before choosing the next implementation.
The standing development goal is an end-to-end usable CAD kernel, not only a
collection of isolated geometry evaluators. Advance the next prerequisite in
the goal's execution order and keep implemented status and limitations honest.

- Implement the geometric core in pure Rust using f64, explicit tolerances,
  documented units, and checked invariants. Share it between native and WASM.
- Preserve exact B-rep geometry, oriented shared topology, and surface pcurves.
  Generate display meshes from B-rep; do not use mesh CSG as exact CAD modeling.
- Do not copy or translate OCCT source. Record mathematical references and
  dependency provenance/licenses. Original code uses MIT OR Apache-2.0.
- Unsupported operations and out-of-domain inputs must return explicit errors.
  Never disguise approximations, placeholders, or no-op results as completion.
- Deliver working code, meaningful edge-case tests, and an example/demo for
  each milestone. Update the roadmap and goal status with verified results.
- Run cargo fmt checks, strict clippy, relevant native tests, and WASM checks
  for kernel changes. Run browser tests when browser behavior changes.
- Make routine design decisions autonomously. Continue toward the standing
  goal when the user asks to continue; report actual progress and remaining
  limitations rather than claiming the entire goal is complete prematurely.
