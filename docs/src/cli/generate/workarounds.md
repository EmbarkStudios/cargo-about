# Workarounds

Here's a list of the current set of workarounds, the crates they apply to, and why the workaround is needed

## `bitvec`

The `bitvec` crate and one of its dependencies by the same author don't include the license information in the crate source.

- [`bitvec`](https://crates.io/crates/bitvec)
- [`wyz`](https://crates.io/crates/wyz)

## `chrono`

The `chrono` crate puts both the `Apache-2.0` and `MIT` license texts in the same file, which confuses `askalono` and also means the SPDX expression is not machine readable.

- [`chrono`](https://crates.io/crates/chrono)

## `cocoa`

Some of the crates published from <https://github.com/servo/core-foundation-rs> do not properly package the license text when publishing the crate.

- [`cocoa-foundation`](https://crates.io/crates/cocoa-foundation)
- [`core-foundation`](https://crates.io/crates/core-foundation)
- [`core-foundation-sys`](https://crates.io/crates/core-foundation-sys)
- [`core-graphics-types`](https://crates.io/crates/core-graphics-types)

## `gtk`

The various `gtk` crates don't include the license text in older versions, though versions published after 2021-10-21 will have the license information in the packaged source so the workaround is not needed for those versions.

- [`atk-sys`](https://crates.io/crates/atk-sys)
- [`cairo-sys-rs`](https://crates.io/crates/cairo-sys-rs)
- [`gdk-pixbuf-sys`](https://crates.io/crates/gdk-pixbuf-sys)
- [`gdk-sys`](https://crates.io/crates/gdk-sys)
- [`gio-sys`](https://crates.io/crates/gio-sys)
- [`glib-sys`](https://crates.io/crates/glib-sys)
- [`gobject-sys`](https://crates.io/crates/gobject-sys)
- [`gtk-sys`](https://crates.io/crates/gtk-sys)

## `ring`

The `ring` crate puts puts 4 different licenses in a single file which confuses tools and also doesn't declare its expression in the Cargo.toml manifest.

- [`ring`](https://crates.io/crates/ring)

## `rustls`

The `rustls` crate puts puts 3 different licenses in a single file which confuses tools. This should be fixed in later versions.

- [`rustls`](https://crates.io/crates/rustls)

## `sentry`

None of the crates published from <https://github.com/getsentry/sentry-rust> include the license text.

- [`sentry-actix`](https://crates.io/crates/sentry-actix)
- [`sentry-anyhow`](https://crates.io/crates/sentry-anyhow)
- [`sentry-backtrace`](https://crates.io/crates/sentry-backtrace)
- [`sentry-contexts`](https://crates.io/crates/sentry-contexts)
- [`sentry-core`](https://crates.io/crates/sentry-core)
- [`sentry-debug-images`](https://crates.io/crates/sentry-debug-images)
- [`sentry-log`](https://crates.io/crates/sentry-log)
- [`sentry-panic`](https://crates.io/crates/sentry-panic)
- [`sentry-slog`](https://crates.io/crates/sentry-slog)
- [`sentry-tower`](https://crates.io/crates/sentry-tower)
- [`sentry-tracing`](https://crates.io/crates/sentry-tracing)
- [`sentry-types`](https://crates.io/crates/sentry-types)
- [`sentry`](https://crates.io/crates/sentry)

## `tonic`

None of the crates published from <https://github.com/hyperium/tonic> include the license text.

- [`tonic`](https://crates.io/crates/tonic)
- [`tonic-build`](https://crates.io/crates/tonic-build)
- [`tonic-health`](https://crates.io/crates/tonic-health)
- [`tonic-types`](https://crates.io/crates/tonic-types)
- [`tonic-reflection`](https://crates.io/crates/tonic-reflection)

## `tract`

None of the crates published from <https://github.com/sonos/tract> included the license text previous to versions 0.15.4. Versions after this do include the license text and this workaround is not needed

- [`tract-data`](https://crates.io/crates/tract-data)
- [`tract-linalg`](https://crates.io/crates/tract-linalg)
- [`tract-core`](https://crates.io/crates/tract-core)
- [`tract-pulse`](https://crates.io/crates/tract-pulse)
- [`tract-pulse-opl`](https://crates.io/crates/tract-pulse-opl)
- [`tract-hir`](https://crates.io/crates/tract-hir)
- [`tract-nnef`](https://crates.io/crates/tract-nnef)
- [`tract-tensorflow`](https://crates.io/crates/tract-tensorflow)
- [`tract-onnx-opl`](https://crates.io/crates/tract-onnx-opl)
- [`tract-onnx`](https://crates.io/crates/tract-onnx)
- [`tract-kaldi`](https://crates.io/crates/tract-kaldi)
- [`tract-cli`](https://crates.io/crates/tract-cli)

## `wasmtime`

The crates around `wasmtime` and `cranelift`, many but not all of which are published from <https://github.com/bytecodealliance/wasmtime>, use the `Apache-2.0 WITH LLVM-exception`, and the license text reflects this. However, neither `clearlydefined.io` nor `askalono` report the inclusion of the `LLVM-exception`, so this workaround just clarifies that.

- [`cranelift-bforest`](https://crates.io/crates/cranelift-bforest)
- [`cranelift-codegen`](https://crates.io/crates/cranelift-codegen)
- [`cranelift-codegen-meta`](https://crates.io/crates/cranelift-codegen-meta)
- [`cranelift-codegen-shared`](https://crates.io/crates/cranelift-codegen-shared)
- [`cranelift-entity`](https://crates.io/crates/cranelift-entity)
- [`cranelift-frontend`](https://crates.io/crates/cranelift-frontend)
- [`cranelift-native`](https://crates.io/crates/cranelift-native)
- [`cranelift-wasm`](https://crates.io/crates/cranelift-wasm)
- [`regalloc`](https://crates.io/crates/regalloc)
- [`wasi-cap-std-sync`](https://crates.io/crates/wasi-cap-std-sync)
- [`wasi-common`](https://crates.io/crates/wasi-common)
- [`wasmparser`](https://crates.io/crates/wasmparser)
- [`wasmtime-environ`](https://crates.io/crates/wasmtime-environ)
- [`wasmtime-jit`](https://crates.io/crates/wasmtime-jit)
- [`wasmtime-runtime`](https://crates.io/crates/wasmtime-runtime)
- [`wasmtime-types`](https://crates.io/crates/wasmtime-types)
- [`wasmtime-wasi`](https://crates.io/crates/wasmtime-wasi)
- [`wast`](https://crates.io/crates/wast)
- [`wiggle`](https://crates.io/crates/wiggle)
- [`wiggle-generate`](https://crates.io/crates/wiggle-generate)
- [`wiggle-macro`](https://crates.io/crates/wiggle-macro)
- [`winx`](https://crates.io/crates/winx)
