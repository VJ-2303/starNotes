# #0001 blaze fails on freetype-sys build script: unsupported writable static __RUST_STD_INTERNAL_VAL

- 2026-10-31T09:14:22Z `issue`: blaze fails on freetype-sys build script: unsupported writable static __RUST_STD_INTERNAL_VAL [freetype-sys v0.20.1]
- 2026-10-31T10:41:14Z `fix`: Use cargo with sccache and mold linker instead of blaze to avoid ra-jit freetype-sys static failure and stay within 7GB RAM [.cargo/config.toml]
