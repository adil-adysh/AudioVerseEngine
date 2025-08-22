# resonance-cxx guidance — how this repo bridges to resonance_audio_api.h

This document records the exact pattern used in `resonance-cxx/` to bridge the upstream
`resonance_audio_api.h` C++ public surface into safe, ergonomic Rust via `cxx`.
Follow these steps when exposing other `.h` files using the same pattern.

Summary of what the repo already does
- Uses `#[cxx::bridge]` to define the shared PODs, enums and the opaque `Api` type in `src/bridge.rs`.
- Provides a small C++ wrapper layer under `resonance-cxx/cxx/include` and `resonance-cxx/cxx/src/`:
  - `resonance_bridge.h` declares a `ra::Api` C++ type that holds a `std::unique_ptr<vraudio::ResonanceAudioApi>` and mirrors the methods exported in the bridge.
  - `resonance_bridge.cc` implements `ra::Api` methods by forwarding calls to `impl_->...` (the upstream vraudio API).
  - `resonance_api_wrapper.cc` adapts an internal `ResonanceAudioApi` wrapper type used by other consumers.
- `build.rs` ensures the upstream C++ library is present (optionally runs CMake), sets linker/search paths, and uses `cxx_build::bridge("src/bridge.rs")` to compile the generated bridge and wrapper C++ sources.

Why this pattern
- Keeps the `cxx::bridge` surface small and Rust-centric (PODs, enums and compact methods).
- Pushes heavy conversions and pointer handling to C++ where upstream types and performance expectations live.
- Keeps ownership and lifetime clear: factories return raw C pointers which the wrapper converts to `std::unique_ptr` and exposes as `UniquePtr<Api>` to Rust.

Step-by-step recipe to expose another C++ header using the same pattern

1) Inspect the upstream header
   - Identify: public factory functions, opaque types, enums, POD structs, buffer APIs (interleaved vs planar), and ownership semantics (who frees what).
   - Note exception behavior (does it throw?), required includes, and whether there are heavy dependencies (Embree, FFTW, etc.).

2) Design the bridge (Rust) surface in `src/bridge.rs`
   - Declare shared PODs and enums exactly (fields, types, ordering). Use fixed-size integer types and `#[repr(i32)]` for enums to match C++ `int`.
   - Declare an opaque `type Api;` inside `unsafe extern "C++"` and the factory function(s) returning `UniquePtr<Api>`.
   - For methods that accept buffers, declare Rust signatures using slices: `fn fill_interleaved_f32(self: Pin<&mut Api>, num_channels: usize, num_frames: usize, buffer: &mut [f32]) -> bool;`.
   - For methods that accept POD structs, reference the identical structs declared in the bridge. For methods that may throw, use `-> Result<T>` on Rust side.

3) Create a C++ header wrapper in `cxx/include/` (e.g., `your_bridge.h`)
   - Forward-declare upstream classes (e.g., `namespace vraudio { class ResonanceAudioApi; }`) to avoid including the heavy header in the generated cxx header.
   - Declare a `ra::Api` C++ class matching the Rust bridge methods (same names, parameter types). Keep it small and header-only (no inline heavy code) so `cxx` can parse it.
   - Add a factory function declaration in the header: `std::unique_ptr<Api> make_...(...)` mirroring the Rust factory declaration.

4) Implement the wrapper in `cxx/src/` (e.g., `your_bridge.cc`)
   - Include `your_bridge.h`, the cxx-generated header `resonance-cxx/src/bridge.rs.h`, and the authoritative upstream header (e.g., `resonance_audio_api.h`).
   - Implement the `ra::Api` constructor to accept a `std::unique_ptr<vraudio::ResonanceAudioApi>` and store it as the `impl_` member.
   - Implement each `ra::Api` method to forward to `impl_->MethodName(...)`.
     - For enums: `static_cast` from `ra::Enum` to `::vraudio::Enum`.
     - For buffers: validate sizes (example: check channels*num_frames == slice.len()) and call the upstream pointer API (`buffer.data()`).
     - For POD structs: copy each member into the upstream struct and call the upstream setter. Keep the field-level mapping explicit (avoid memcpy unless layouts are guaranteed identical).
   - Implement the factory `make_api(...)` that calls the upstream factory (e.g., `vraudio::CreateResonanceAudioApi`) and wraps the returned raw pointer into `std::unique_ptr<vraudio::ResonanceAudioApi>` and then into `std::unique_ptr<ra::Api>`.

5) Update `build.rs` to compile and link
   - Ensure `build.rs` lists `println!("cargo:rerun-if-changed=src/bridge.rs");` and the C++ wrapper sources and headers.
   - Try to configure/build the upstream C++ library (using CMake) if the repository contains the upstream code. Otherwise allow developers to set `VRAUDIO_LIB_DIR` and `VRAUDIO_LIB_NAME` to point to prebuilt artifacts.
   - Use `let mut build = cxx_build::bridge("src/bridge.rs"); build.file("cxx/src/your_bridge.cc"); build.include("cxx/include");`.
   - Add `build.include(out_dir)` and `build.include(out_dir.join("cxxbridge").join("include"))` so cxx-generated headers are found. Also add upstream include dirs (e.g., `resonance-audio/resonance_audio/api`).
   - Set `println!("cargo:rustc-link-search=native={}", native_lib_dir.display()); println!("cargo:rustc-link-lib=static={}", lib_name);`

6) Run `cargo build` and iterate
   - If CMake is available, `build.rs` may run it to create the upstream library. Otherwise ensure `VRAUDIO_LIB_DIR` points to a directory containing `YourLibName.lib` (Windows) or `.a`/`.so` on other platforms.
   - Fix include path or missing symbols by adding the right `build.include(...)` and `build.file(...)` entries.

Practical tips and gotchas (accurate details observed in repo)
- Generated header include path: cxx writes headers under `$OUT_DIR/cxxbridge/include`; add that path to `cxx_build::bridge(...).include(...)` so `#include "resonance-cxx/src/bridge.rs.h"` resolves.
- Keep `cxx/include/resonance_bridge.h` minimal and only forward-declare types used in the RA class to avoid circular includes in the generated header.
- Ownership: upstream factory returns raw pointer. Immediately wrap it in `std::unique_ptr<UpstreamType>` in the wrapper code so the `ra::Api` destructor can safely delete.
- Buffer validation: always check `channels * frames == slice.size()` before calling upstream functions. Print an error and return safely otherwise.
- Enum boundaries: when mapping enums, be defensive — although `cxx` maps enums, upstream may add values; keep code explicit (static_cast) and consider validating ranges if possible.
- POD mapping: map fields individually rather than memcpy to avoid surprises from padding or layout changes.
- Planar APIs: C++ APIs that take `T* const*` require building an array of per-channel pointers. Implement that in C++ wrapper before forwarding to upstream API.
- Exceptions: If upstream C++ functions may throw, either wrap calls in try/catch and translate to `cxx::Exception` (declare Result<T> in the bridge) or ensure the upstream doesn't throw in hot paths.

Testing & CI
- Provide a lightweight placeholder upstream implementation (small C++ file implementing the expected factory and methods) to link against during early development. This allows the Rust side to compile before the real library is available.
- Add a CI job that builds the upstream C++ (via CMake) and then runs `cargo build` to validate the full native+bridge build.

Example checklist when adding a new header `foo.h`
- [ ] Create `src/bridge.rs` entries for `Foo` types and `UniquePtr<Api>` factory.
- [ ] Add `cxx/include/foo_bridge.h` with `ra::Api` declaration.
- [ ] Implement `cxx/src/foo_bridge.cc` forwarding to `::vraudio::FooApi`.
- [ ] Add files to `build.rs` (`bridge.rs` rerun-if-changed, .file entries, include paths).
- [ ] Ensure upstream header include path is added to `build.include(...)`.
- [ ] Run `cargo build` and fix missing link/include errors.

Further reading (in-repo)
- `resonance-cxx/src/bridge.rs` — the Rust bridge declarations used by the crate.
- `resonance-cxx/cxx/include/resonance_bridge.h` — forward-declare + Api class declaration.
- `resonance-cxx/cxx/src/resonance_bridge.cc` — implementation forwarding to the upstream API.
- `resonance-cxx/build.rs` — build orchestration (CMake, link dirs, cxx_build invocation).

If you want, I can now:
- Generate a scaffold `cxx/include/foo_bridge.h` + `cxx/src/foo_bridge.cc` + `src/bridge.rs` entries for a chosen header and iterate until `cargo build` succeeds.
- Add a CI job snippet for building the C++ upstream and the Rust bridge.
