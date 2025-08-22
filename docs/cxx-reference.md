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


## cxx bridge reference (AudioVerseEngine)

This file documents the minimal, practical pattern we use across this repository to expose selected C++ APIs to Rust using the `cxx` crate. It is intentionally short and focused: follow the checklist below, then consult `docs/resonance_cxx_guidance.md` for worked examples and troubleshooting.

Key goals

- Keep the Rust-facing `#[cxx::bridge]` small and explicit: declare only POD structs, enums, and an opaque `Api` type.
- Implement a thin C++ wrapper (in `cxx/include` + `cxx/src`) that owns the real upstream object and forwards calls while performing simple validation and lightweight conversions.
- Centralize build and linking decisions in `build.rs`. Allow using prebuilt upstream artifacts (via environment variables) or build them locally with CMake when necessary.

When to use this pattern

- Use it for stable, well-understood APIs where copying small PODs across the boundary is sufficient.
- Avoid passing large or frequently-updated buffers across the bridge; instead, design a controlled API that accepts slices for one-time transfers or uses shared memory patterns.

Checklist to add a new C++ API to Rust

1) Design the Rust surface

  - Create a minimal `#[cxx::bridge]` module that exposes only the types and functions you need. Example minimal bridge:

    - Declare POD structs and enums with identical field layouts in Rust and C++.
    - Declare an opaque `type Api;` and a factory that returns `UniquePtr<Api>`.

  - Keep error handling simple: return booleans or numeric error codes, or use a small POD `Result` struct that contains an error code + optional message.

2) Add C++ wrapper header (cxx/include)

  - Add a header (example: `cxx/include/myapi_bridge.h`) that declares a wrapper namespace/class, e.g. `namespace myapi { class Api { ... }; }` and an extern factory function `std::unique_ptr<myapi::Api> make_api(...);`.

  - The wrapper header should include only lightweight upstream headers or forward-declare upstream types. Keep it stable and small so cxx generated headers don't pull large transitive includes.

3) Implement wrapper (cxx/src)

  - Implement the wrapper class to own a `std::unique_ptr<UpstreamApi>` (or other RAII wrapper) and forward calls.

  - Validate inputs and buffer sizes on the C++ side. For example, when a Rust function sends an interleaved slice with `frames * channels` floats, assert that the slice length matches and return an error if it doesn't.

  - Convert enums and map POD fields explicitly. Don't assume identical internal representations unless it's trivial and documented.

4) Rust-side helpers

  - Implement small Rust helper functions that convert idiomatic Rust types (Vec, &str) into the PODs expected by the cxx bridge.

  - Keep copying predictable: copy small vectors/slices into the cxx-safe types and avoid leaking raw pointers across threads.

5) build.rs and linking

  - The crate's `build.rs` should handle two modes:
    - Use prebuilt upstream artifacts when environment variables like `UPSTREAM_LIB_DIR` and `UPSTREAM_LIB_NAME` are set.
    - Otherwise, try to invoke CMake (or the upstream project's build) to build the required static/dynamic library into `OUT_DIR`.

  - After locating/building the library, emit `cargo:rustc-link-search=native=...` and `cargo:rustc-link-lib=static|dylib=...` as appropriate.

  - Use `cxx_build::bridge("src/lib.rs")` to build the generated glue sources and include the `OUT_DIR` path for headers if needed.

6) Tests and CI

  - Add unit tests that exercise the Rust-facing API. Use small, deterministic inputs and verify outputs.

  - If building upstream C++ is expensive, provide prebuilt artifacts for CI or use a small mock implementation behind the same wrapper for fast tests.

Practical examples and notes

- Wrapper ownership: prefer `std::unique_ptr<UpstreamApi>` inside the wrapper class and return `UniquePtr<Api>` to Rust. This matches Rust ownership semantics and keeps the bridge logic trivial.

- POD alignment: ensure both sides declare fields in the same order and with compatible primitive types. Use static asserts on the C++ side when helpful.

- Buffer validation: check `channels * frames == slice.len()` before accessing the buffer contents.

- Enum mapping: if an upstream enum changes, update both the C++ wrapper and the Rust enum; keep a mapping function in the wrapper implementation.

Common pitfalls

- Pulling large upstream headers into `cxx::bridge` causes long compile times and fragile dependency graphs; prefer forward declarations in the wrapper header.
- Returning non-POD types across the bridge or capturing STL containers directly is unsupported; convert to PODs or `UniquePtr`/`String` helpers.
- Relying on implicit integer sizes (e.g., `int`) can break portability; pick explicit-sized types (`int32_t`, `uint64_t`) in C++ and matching Rust types.

References and further reading

- See `docs/resonance_cxx_guidance.md` for a step-by-step example that follows this pattern in this repository.
- Inspect the `resonance-cxx` crate in the workspace to see a production example of this pattern: `cxx/include/resonance_bridge.h` and `cxx/src/resonance_bridge.cc`.

Quick checklist (copyable)

- [ ] Add `#[cxx::bridge]` with PODs/enums and `type Api;`.
- [ ] Add `cxx/include/<api>_bridge.h` with wrapper declaration and factory.
- [ ] Implement `cxx/src/<api>_bridge.cc` forwarding to upstream API and performing validations.
- [ ] Update `build.rs` to find/build upstream lib and call `cxx_build::bridge(...)`.
- [ ] Add Rust helper helpers for conversions and small unit tests.

License

This reference is specific to the AudioVerseEngine repo and reflects conventions used by the maintainers.
- Ensure `println!("cargo:rerun-if-changed=...")` is present in `build.rs` for bridge.rs, headers, and .cc files.
- If C++ functions are declared in the bridge but not implemented in C++ files, linking will fail with undefined references.
- When developing without the external library, you can provide lightweight placeholder implementations in C++ that mimic signatures to allow linking during early development.

Useful references
- cxx book/examples: https://github.com/dtolnay/cxx/tree/master/book
- bridge attribute details: #[cxx::bridge]

End of cxx reference.
