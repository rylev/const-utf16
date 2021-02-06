[![crates.io](https://img.shields.io/crates/v/const-utf16.svg)](https://crates.io/crates/const-utf16)
[![docs.rs](https://docs.rs/const-utf16/badge.svg)](https://docs.rs/const-utf16/)
[![Build and Test](https://github.com/rylev/const-utf16/workflows/Build%20and%20Test/badge.svg?event=push)](https://github.com/rylev/const-utf16/actions)

# const-utf16

utf8 to utf16 conversion functions useable in const contexts. 

## Use

```rust
const HELLO_WORLD_UTF16: const_utf16::Utf16Buffer = const_utf16::encode_utf16("Hello, world!");
```

# Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.46.0 or newer due to the use of some const expression features.

## Attribution

This code is largely inspired by the [Rust core utf16 conversion code](https://github.com/rust-lang/rust/blob/399b6452b5d9982438be208668bc758479f13725/library/core/src/char/methods.rs#L1627-L1652).