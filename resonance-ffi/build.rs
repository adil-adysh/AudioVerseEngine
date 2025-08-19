use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=../resonance-audio/resonance_audio/api/resonance_audio_api.h");

    // Ensure native resonance-audio is built fresh so linker symbols are available.
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let resonance_audio_dir = workspace_root.join("resonance-audio");
    let build_dir = resonance_audio_dir.join("build");

    // Remove previous build to force a fresh CMake configure/build.
    if build_dir.exists() {
        println!("Removing existing native build directory: {}", build_dir.display());
        let _ = fs::remove_dir_all(&build_dir);
    }

    // Configure with CMake from the resonance-audio directory.
    let mut cmake_config = Command::new("cmake");
    cmake_config.current_dir(&resonance_audio_dir)
        .arg("-S").arg(".")
        .arg("-B").arg("build")
        .arg("-DSTATIC_MSVC_RUNTIME=OFF")
        .arg("-DBUILD_RESONANCE_AUDIO_API=ON");

    // Add FFI sources argument (CMake variable) pointing to FFI files in this crate.
    let ffi_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ffi_sources = format!("{}/resonance_c_api.cc;{}/resonance_c_api.h", ffi_dir.display(), ffi_dir.display());
    cmake_config.arg(format!("-DFFI_SOURCES={}", ffi_sources.replace('\\', "/")));

    let status = cmake_config.status().expect("Failed to spawn cmake for configure");
    if !status.success() {
        panic!("CMake configure failed");
    }

    // Build Release configuration
    let status = Command::new("cmake")
        .current_dir(&resonance_audio_dir)
        .arg("--build").arg("build")
        .arg("--config").arg("Release")
        .status()
        .expect("Failed to spawn cmake for build");
    if !status.success() {
        panic!("CMake build failed");
    }

    // Emit link search and link lib so rustc can link the produced native library.
    // The CMake build places libraries under: resonance-audio/build/resonance_audio/Release
    let native_lib_dir = resonance_audio_dir.join("build").join("resonance_audio").join("Release");
    println!("cargo:rustc-link-search=native={}", native_lib_dir.display());
    // Prefer static library produced by the build (ResonanceAudioStatic.lib -> link name: ResonanceAudioStatic)
    println!("cargo:rustc-link-lib=static=ResonanceAudioStatic");

    // --- Now generate bindings using bindgen ---
    let msvc_include = "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/include";
    let windows_sdk_include = "C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0/ucrt";
    let llvm_include = "C:/Program Files/LLVM/include";

    let bindings = bindgen::Builder::default()
        // Use the C wrapper header which exposes a C ABI over the C++ API.
        .header("wrapper.h")
        // Include paths for any headers referenced by wrapper.h (if needed)
        .clang_arg(format!("-I{}", msvc_include))
        .clang_arg(format!("-I{}", windows_sdk_include))
        .clang_arg(format!("-I{}", llvm_include))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_path.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");

    // Patch extern "C" blocks to unsafe extern "C" and strip bindgen #[test] layout checks
    let content = fs::read_to_string(&bindings_path).expect("Couldn't read generated bindings");
    let patched = content.replace("extern \"C\"", "unsafe extern \"C\"");

    // Remove bindgen-generated #[test] functions to avoid compiling tests into the rlib
    let mut out = String::with_capacity(patched.len());
    let mut lines = patched.lines();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("#[test]") {
            // skip the #[test] line and subsequent fn bindgen_test_layout_* block
            // assume the test function starts with `fn bindgen_test_layout_` and ends with a `}` on its own line
            while let Some(l) = lines.next() {
                if l.trim_end().ends_with('}') {
                    break;
                }
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    fs::write(&bindings_path, out).expect("Couldn't patch bindings");
}
