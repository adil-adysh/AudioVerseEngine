---
applyTo: '**'
---

# resonance-cxx bridge — AI-friendly instructions

Purpose
- Provide a compact, actionable guide for automated agents (or humans using AI assistants) to add or update a cxx bridge that exposes an upstream C++ header into Rust using the same pattern already used in `resonance-cxx/`.

When to use
- You're adding a new C++ header (factory + opaque types + PODs/enums) to the Rust workspace and want a minimal, safe `cxx` bridge.

High-level contract (2–4 bullets)
- Inputs: an upstream C++ header that exposes factory functions, opaque types, POD structs, enums, and buffer APIs.
- Outputs: a small Rust `#[cxx::bridge]` surface (`src/bridge.rs`) and a thin C++ wrapper (`cxx/include/*` + `cxx/src/*`) that forwards calls to the upstream API and returns `UniquePtr<Api>` factories.
- Error modes: missing includes, link errors, or mismatched POD layouts; treat them as build failures that need additional include paths or a small placeholder upstream implementation for CI.

Checklist (explicit actionable steps)
1. Inspect upstream header
   - Identify public factory functions, opaque types, enums, POD structs, buffer APIs (interleaved vs planar), exception behavior, and ownership semantics.

2. Design Rust bridge (`resonance-cxx/src/bridge.rs`)
   - Declare PODs and enums with fixed-size types and matching ordering.
   - Use `#[repr(i32)]` for enums that map to C++ `int`.
   - Declare `type Api;` inside `unsafe extern "C++"` and a factory returning `UniquePtr<Api>`.
   - Prefer Rust slice signatures for buffers (example: `&mut [f32]`) and `Result<T>` for fallible calls.

3. Create C++ header wrapper (`resonance-cxx/cxx/include/your_bridge.h`)
   - Forward-declare heavy upstream types to avoid pulling large headers into cxx-generated headers.
   - Declare a minimal `ra::Api` class that mirrors the Rust bridge method names and types.
   - Add a factory declaration `std::unique_ptr<Api> make_*` matching Rust factory names.

4. Implement wrapper (`resonance-cxx/cxx/src/your_bridge.cc`)
   - Include the wrapper header, the cxx-generated header (`resonance-cxx/src/bridge.rs.h`), and the authoritative upstream header.
   - Implement `ra::Api` methods to forward to `impl_->...`.
   - Validate buffer sizes (e.g., `channels * frames == buffer_len`) and convert slices/pointers appropriately.
   - Map enums with `static_cast` and copy POD fields individually.
   - Wrap raw upstream pointers into `std::unique_ptr<UpstreamType>` immediately in factory code.

5. Update `build.rs`
   - Add `println!("cargo:rerun-if-changed=src/bridge.rs");`.
   - Use `cxx_build::bridge("src/bridge.rs")`, add `.file("cxx/src/your_bridge.cc")`, and `.include("cxx/include")`.
   - Add cxx-generated header include path (`$OUT_DIR/cxxbridge/include`) and upstream include dirs.
   - If the repo contains upstream C++ sources, optionally run CMake here; otherwise allow `VRAUDIO_LIB_DIR`/`VRAUDIO_LIB_NAME` env vars to point at prebuilt artifacts.

6. Build & iterate
   - Run `cargo build`. If there are missing symbols or include errors, add include paths and files to `build.rs` and re-run.
   - For early development, add a lightweight placeholder implementation of the upstream API to link against so the Rust bridge compiles before the real library is available.

Practical tips & gotchas (short)
- Add `$OUT_DIR/cxxbridge/include` to cxx build includes so `#include "resonance-cxx/src/bridge.rs.h"` resolves.
- Keep `cxx/include/*` headers minimal and only forward-declare upstream types when possible.
- Validate buffer sizes before forwarding and return safely on mismatch.
- Map POD fields individually to avoid padding/layout surprises.
- For planar buffer APIs (`T* const*`), build a per-channel pointer array in C++ before calling upstream.
- Wrap calls that may throw in try/catch and translate to `Result<T>` on the Rust side.

Testing & CI
- Add a tiny placeholder C++ implementation file that implements the expected upstream factory and methods so CI can link and compile the bridge even before the full upstream library is available.
- Add a CI job that builds the upstream native library (via CMake when possible) and then runs `cargo build` to validate the end-to-end native+bridge build.

Example quick checklist for `foo.h`
- Create `resonance-cxx/src/bridge.rs` entries for Foo types and a `UniquePtr<Api>` factory.
- Add `resonance-cxx/cxx/include/foo_bridge.h` with `ra::Api` declaration.
- Implement `resonance-cxx/cxx/src/foo_bridge.cc` forwarding to `::vraudio::FooApi`.
- Add files and include dirs to `resonance-cxx/build.rs`.
- Run `cargo build` and fix any link/include issues.

Where to look in this repo
- `resonance-cxx/src/bridge.rs`
- `resonance-cxx/cxx/include/resonance_bridge.h`
- `resonance-cxx/cxx/src/resonance_bridge.cc`
- `resonance-cxx/build.rs`

Next steps the agent can take (optional)
- Generate scaffolding: `cxx/include/foo_bridge.h`, `cxx/src/foo_bridge.cc`, and `src/bridge.rs` entries for a chosen header and iterate until `cargo build` succeeds.
- Create a CI snippet that builds the upstream C++ with CMake and then runs `cargo build` in CI.

Quality gates
- Build: cargo build (should succeed after adding include/link paths)
- Tests: add unit tests or a placeholder implementation to validate bridge compilation early

End of instructions.
