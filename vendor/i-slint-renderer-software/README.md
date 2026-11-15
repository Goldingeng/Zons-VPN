
**NOTE**: This library is an **internal** crate of the [Slint project](https://slint.dev).
This crate should **not be used directly** by applications using Slint.
You should use the `slint` crate instead.

**WARNING**: This crate does not follow the semver convention for versioning and can
only be used with `version = "=x.y.z"` in Cargo.toml.

**Modified for Zons VPN**: glyph rasterization changed in `fonts/vectorfont.rs` and `draw_glyph_run` in `lib.rs` (TrueType hinting, quarter-pixel glyph positions, gamma 1.4).
