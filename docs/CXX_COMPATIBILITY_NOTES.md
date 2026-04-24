# C++ Compatibility Notes

This file records compatibility details discovered while aligning the Rust
decoder with the C++ Draco decoder. These are not necessarily ideal arithmetic
or hardening choices; they are observable C++ semantics that existing `.drc`
streams may depend on.

## Portable TEXCOORD Prediction Arithmetic

Source area:

- C++: `src/draco/compression/attributes/prediction_schemes/mesh_prediction_scheme_tex_coords_portable_predictor.h`
- Rust: `crates/draco-core/src/prediction_scheme_tex_coords_portable.rs`

The portable TEXCOORD decoder uses unsigned arithmetic in the orientation branch
to avoid signed overflow while reconstructing the predicted UV value. The C++
flow is subtle:

1. Compute `Vec2u(x_uv) +/- Vec2u(cx_uv)` using unsigned wrapping arithmetic.
2. Convert the wrapped unsigned vector back to signed `Vec2<int64_t>`.
3. Divide the signed vector by `pn_norm2_squared`.

Rust must preserve that cast order. Dividing as unsigned first and then casting
back to signed changes decoded UVs for wrapped negative intermediates.

The same predictor also computes:

```cpp
IntSqrt(cx_norm2_squared * pn_norm2_squared)
```

where both operands are unsigned. In C++ this multiplication can wrap before
`IntSqrt()`. The Rust decoder preserves that wrapping behavior for C++ parity;
using a checked multiply rejected at least one existing v2.2 Edgebreaker fixture
that the C++ decoder accepts.

## Potential C++ Improvement

If the C++ implementation is hardened in the future, these two spots are worth
reviewing:

- whether the unsigned orientation add/sub should document the cast-before-divide
  behavior explicitly;
- whether the `cx_norm2_squared * pn_norm2_squared` multiplication should be
  guarded, widened, or documented as intentionally wrapping.

Changing either behavior in C++ could affect byte-for-byte decode compatibility
with existing streams, so any improvement should be gated by compatibility tests
against historical fixtures.
