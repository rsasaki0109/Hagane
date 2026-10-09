# Binary64 preservation in JSON model input

Hagane enables `serde_json`'s `float_roundtrip` feature for both native and
WebAssembly builds. Finite `f64` values serialized by Rust and read back into
typed values or JSON `Value` preserve their original bits, including subnormal
values and signed zero. No new dependency or geometry approximation is added.

The previous default parser could turn the decimal `0.9999999999999999` into
`1.0`: expected bits `0x3fefffffffffffff`, observed `0x3ff0000000000000`.
Another model coordinate, `0.21827018629739742`, changed from
`0x3fcbf04707eb693e` to `0x3fcbf04707eb693f`. These differences affected
STEP byte reproducibility when native JSON input and browser numeric transport
were intended to represent the same values.

The parser now agrees with independent `f64::from_str` references for the
serialized finite inputs. Tests exercise varied exponent ranges, selected extrema,
subnormals, signed zero, deterministic random finite bit patterns, nested JSON
and trigonometric polygon coordinates. A real 16-corner graph solid retains
its coordinate bits, centroid and exact STEP bytes after JSON reconstruction.
Native/WASM STEP tests use the original non-dyadic trigonometric inputs.

STEP affine pcurve directions also use a scaled square-root norm rather than
platform `hypot`. This avoids an independent native/WASM rounding difference
in direction and vector records while retaining overflow-safe scaling. The
regression compares complete STEP bytes without weakening the comparison or
replacing the trigonometric fixture with rounded coordinates.

The editable browser workflow also checks a trigonometric 16-gon extrusion
through import, exported JSON download and autosave restoration, preserving its
numeric coordinates and analytical volume. Geometry and conditioning checks
still apply after parsing; preserving a value does not make an unsupported
shape valid.

This guarantees the tested binary64 round trip through Hagane's Rust JSON
parser/serializer. JSON numbers are not arbitrary-precision CAD coordinates.
Other languages may serialize their values differently; JavaScript's
`JSON.stringify` turns negative zero into zero and nonfinite numbers into null.
This feature cannot recover information already removed by an upstream writer.
Nonfinite or out-of-range geometry continues to return explicit errors.

Validation: 651 native tests, formatting, strict Clippy, the WASM build and
the complete native/WASM regression passed. The original trigonometric polygon
STEP fixture compares full output bytes, including affine pcurve directions.
The full browser regression passed, including exact coordinate equality after
workflow JSON import, download and autosave restoration.
