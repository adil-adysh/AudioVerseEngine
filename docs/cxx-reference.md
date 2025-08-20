## cxx reference — patterns and signatures to use for the Resonance Audio bridge

Treat `resonance-audio/resonance_audio/api/resonance_audio_api.h` as the source of truth for API shapes and function names.

This document records the cxx patterns, method signatures and rules I used (and will use) when implementing the `resonance-cxx` crate bridge.

## High-level rules

- Use `#[cxx::bridge]` to declare the shared types and the `unsafe extern "C++"` functions/types the C++ side must implement.
- Prefer `UniquePtr<T>` for C++ factory returns that transfer ownership to Rust.
- Use `Pin<&mut T>` on Rust-side `UniquePtr` pinned references when calling mutating C++ methods that may keep internal pointers/aliases.
- Map C++ POD structs (plain arrays + scalars) to identical Rust `struct` definitions inside the `#[cxx::bridge]` module so the generated `.rs.h` header defines the exact layout.
- Pass audio buffers as `&[T]` / `&mut [T]` on Rust side and as `rust::Slice<const T>` / `rust::Slice<T>` on the C++ side. Always validate slice length = `num_channels * num_frames`.
- For enums shared with C++, declare them with `#[repr(i32)]` in Rust and keep the same numeric discriminants as the C++ header. When matching shared enums in Rust, include a wildcard `_ =>` arm to handle out-of-range values.
- When a C++ function may throw, declare it in the bridge to return `Result<T>`; cxx converts thrown `std::exception` into `cxx::Exception` on Rust side.
- If you need `UniquePtr<A>` instantiation for a type `A`, add `impl UniquePtr<A> {}` inside the same `#[cxx::bridge]` so the template is emitted.

## Build / include patterns

- `build.rs` should use `cxx_build::bridge("src/bridge.rs")` and then add `.file(...)` for local C++ source files (e.g. `cxx/src/resonance_bridge.cc`). Use `.include("cxx/include")` and `.include(VRAUDIO_INCLUDE)` for the external library headers.
- Set `println!("cargo:rustc-link-search=native={}", lib_dir)` and `println!("cargo:rustc-link-lib={}", lib_name)` in `build.rs` if linking to the prebuilt Resonance Audio library.
- If you need to re-export headers for downstream crates, use `cxx_build::CFG.exported_header_dirs` / `exported_header_prefixes` / `exported_header_links` in `build.rs`.
- If you prefer a custom include prefix for crate headers, set `cxx_build::CFG.include_prefix` in `build.rs`.

## Common cxx types and how they map

- Rust `UniquePtr<Api>`  <->  C++ `std::unique_ptr<Api>`
- Rust `&CxxString` / `String`  <->  C++ `rust::String` / `const std::string&`
- Rust `&[T]` / `&mut [T]`  <->  C++ `rust::Slice<const T>` / `rust::Slice<T>`
- Rust `Pin<&mut Api>`  <->  C++ methods declared `Api&` (callable as `self: Pin<&mut Api>` in bridge)
- Rust `Result<T>`  <->  C++ functions that may throw; exceptions are converted to `cxx::Exception`.

## Safety notes / best practices

- Always validate slice lengths on the Rust side before calling into C++.
- Use pinned `&mut` only for methods that mutate shared C++ state; call `self.inner.pin_mut()` on `UniquePtr` to obtain `Pin<&mut Api>`.
- Mirror array sizes exactly (e.g. `room_position: [f32;3]`) so layout matches the C++ POD.
- Keep the C++ wrapper thin: own the real `vraudio::ResonanceAudioApi` using `std::unique_ptr` and forward calls to it. Do C++-side conversions between generated `ReflectionProperties`/`ReverbProperties` and the library's types.

## ResonanceAudioApi -> bridge mapping (summary)

The following entries are taken from `resonance_audio_api.h` and show how each API maps to the cxx bridge declaration and expected C++ wrapper behavior.

- Factory
  - C++: extern "C" ResonanceAudioApi* CreateResonanceAudioApi(size_t num_channels, size_t frames_per_buffer, int sample_rate_hz);
  - Bridge: `fn make_api(num_channels: usize, frames_per_buffer: usize, sample_rate_hz: i32) -> UniquePtr<Api>;` (C++ wrapper should call CreateResonanceAudioApi and wrap the raw pointer into a `std::unique_ptr<Api>` or construct an `Api` that owns the `unique_ptr<vraudio::ResonanceAudioApi>`.)

- Output rendering
  - C++: virtual bool FillInterleavedOutputBuffer(size_t num_channels, size_t num_frames, float* buffer_ptr) = 0;
  - Bridge Rust: `fn fill_interleaved_f32(self: Pin<&mut Api>, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool;`
  - Bridge C++: receive `rust::Slice<float>` and call `FillInterleavedOutputBuffer(num_channels, num_frames, buffer.data())`. Validate `buffer.size() == num_channels * num_frames`.

- Planar formats
  - C++ API has `FillPlanarOutputBuffer` and `SetPlanarBuffer` that take `T* const*` (pointer-to-pointer). Planar helpers are best implemented in C++ by building a temporary array of pointers to channel buffers and calling into the library.

- Listener / global state
  - e.g. `SetHeadPosition`, `SetHeadRotation`, `SetMasterVolume`, `SetStereoSpeakerMode`
  - Bridge: one-to-one wrappers taking primitives; pass through to `api.get()->Set...(...)`.

- Sources lifecycle
  - Create: `SourceId CreateAmbisonicSource(size_t num_channels)` -> Bridge returns `i32`.
  - Destroy: `void DestroySource(SourceId id)` -> Bridge `fn destroy_source(self: Pin<&mut Api>, source_id: i32);`

- Source buffers
  - `SetInterleavedBuffer(SourceId, const float*, size_t num_channels, size_t num_frames)` -> in bridge expose `fn set_interleaved_buffer_f32(self: Pin<&mut Api>, source_id: i32, audio: &[f32], num_channels: usize, num_frames: usize);`
  - On C++ side, validate slice length and call `SetInterleavedBuffer(source_id, audio.data(), num_channels, num_frames);`

- Source params
  - Methods like `SetSourcePosition`, `SetSourceVolume`, `SetSourceRotation`, `SetSoundObjectDirectivity`, etc. map to simple pass-through bridge methods taking primitives.

- Environment
  - `EnableRoomEffects(bool)`, `SetReflectionProperties(const ReflectionProperties&)`, `SetReverbProperties(const ReverbProperties&)` map to bridge methods. On C++ side convert the generated `ReflectionProperties` into `vraudio::ReflectionProperties` then call library method.

## Checklist for implementation

- [x] Use `resonance_audio_api.h` as source of truth for method names and parameter orders.
- [x] Expose factories as `UniquePtr<Api>`.
- [x] Expose buffer methods as slice-based Rust signatures and `rust::Slice` on the C++ side.
- [x] Mirror enums and POD structs exactly in the bridge `#[cxx::bridge]`.

---

If you want, I'll now implement the `resonance-cxx` crate files (Rust bridge, `cxx/include/resonance_bridge.h`, `cxx/src/resonance_bridge.cc`, `build.rs`, Cargo.toml) using the header above as authoritative and then run `cargo build` to iterate until it compiles. Proceeding next unless you prefer to review this reference first.
cxx reference
=================

This file summarizes the important `cxx` rules, method signatures, and patterns to follow when implementing the `resonance-cxx` crate.

Key points
- Add `cxx = "1.0"` to `[dependencies]` and `cxx-build = "1.0"` to `[build-dependencies]`.
- Use `#[cxx::bridge]` to declare shared types and `unsafe extern "C++"` blocks for C++ functions/opaque types.
- For C++ `std::unique_ptr<T>` return types use `UniquePtr<T>` on Rust side.
- Opaque C++ types are declared as `type Api;` within the `unsafe extern "C++"` block.
- For functions that accept/return slices use `rust::Slice<T>` on C++ side and `&[T]`/`&mut [T]` on Rust side.
- For `std::string` interoperability use `&CxxString` / `String` and include the generated header.
- When declaring enums with numeric layout, use `#[repr(i32)]` on Rust side to match C++ `int` sized enums. Always provide a `_`/fallback arm when matching shared enums.
- For `UniquePtr<T>` template instantiation across modules, add `impl UniquePtr<T> {}` in the bridge where `T` is declared if needed.
- Use `cxx_build::bridge("src/bridge.rs")` in `build.rs` and add `.file(...)` for C++ implementation files. Use `.std("c++17")` or `.flag_if_supported("-std=c++17")` to enable the desired standard.
- Use `CFG.include_prefix` or `CFG.exported_header_dirs`/`CFG.exported_header_prefixes` if you need custom include paths or to re-export headers to downstream crates.
- For functions returning `Result<T>`, declare `Result<T>` in the bridge and handle `cxx::Exception` on the Rust side; C++ exceptions are converted to `cxx::Exception`.

Common type mappings
- Rust & C++ slice: Rust `&[T]` / `&mut [T]` <-> C++ `rust::Slice<const T>` / `rust::Slice<T>`
- Rust Vec<T> <-> C++ `rust::Vec<T>` (C++ can iterate using range-for)
- Rust &str / String <-> C++ `rust::Str` / `rust::String` / `const std::string&`
- Rust UniquePtr<T> <-> C++ `std::unique_ptr<T>`

Build tips
- Ensure `println!("cargo:rerun-if-changed=...")` is present in `build.rs` for bridge.rs, headers, and .cc files.
- If C++ functions are declared in the bridge but not implemented in C++ files, linking will fail with undefined references.
- When developing without the external library, you can provide lightweight placeholder implementations in C++ that mimic signatures to allow linking during early development.

Useful references
- cxx book/examples: https://github.com/dtolnay/cxx/tree/master/book
- bridge attribute details: #[cxx::bridge]

End of cxx reference.
