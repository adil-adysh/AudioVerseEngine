// File: build.rs
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::fs;

fn main() {
    // Rerun the build script if any of these files change.
    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=cxx/include/resonance_bridge.h");
    println!("cargo:rerun-if-changed=cxx/src/resonance_bridge.cc");

    // Get the workspace root and the path to the resonance-audio library.
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let resonance_audio_dir = workspace_root.join("resonance-audio");
    let build_dir = resonance_audio_dir.join("build");

    println!("cargo:warning=--- Starting Native C++ Build Process ---");

    // --- Phase 1: Verify all required files and directories exist ---
    println!("cargo:warning=1. Verifying project structure and required files...");

    let bridge_rs = PathBuf::from("src/bridge.rs");
    let cpp_src_file = PathBuf::from("cxx/src/resonance_bridge.cc");
    let cpp_header_dir = PathBuf::from("cxx/include");
    let cpp_header_file = cpp_header_dir.join("resonance_bridge.h");
    let resonance_audio_api_h = resonance_audio_dir.join("resonance_audio").join("api").join("resonance_audio_api.h");

    assert!(resonance_audio_dir.exists(), "Error: The 'resonance-audio' directory was not found at {}. Please check your project structure.", resonance_audio_dir.display());
    assert!(bridge_rs.exists(), "Error: The Rust bridge file 'src/bridge.rs' was not found.");
    assert!(cpp_src_file.exists(), "Error: The C++ source file 'cxx/src/resonance_bridge.cc' was not found.");
    assert!(cpp_header_dir.exists(), "Error: The C++ header directory 'cxx/include' was not found.");
    assert!(cpp_header_file.exists(), "Error: The C++ header file 'cxx/include/resonance_bridge.h' was not found.");
    assert!(resonance_audio_api_h.exists(), "Error: The required C++ header file 'resonance_audio_api.h' was not found at {}. This file is essential for the C++ build.", resonance_audio_api_h.display());

    println!("cargo:warning=✅ All required files and directories found.");

    // --- Phase 2: Compile the native C++ library using CMake ---
    let _ = fs::create_dir_all(&build_dir);

    println!("cargo:warning=2. Attempting to run CMake to configure C++ build in directory: {}", resonance_audio_dir.display());

    // If CMake is available on PATH, run configure/build. Otherwise, skip
    // and rely on any existing build artifacts under `resonance-audio/build`.
    match Command::new("cmake").arg("--version").status() {
        Ok(_) => {
            println!("cargo:warning=2a. CMake found; running configure...");
            let status = Command::new("cmake")
                .current_dir(&resonance_audio_dir)
                .arg("-S").arg(".")
                .arg("-B").arg("build")
                .arg("-DBUILD_RESONANCE_AUDIO_API=ON")
                .status();
            if let Ok(s) = status {
                if !s.success() {
                    panic!("CMake configure failed. See output for details.");
                }
            } else {
                panic!("Failed to run CMake configure step. Is CMake in your PATH?");
            }
            println!("cargo:warning=✅ CMake configuration complete.");

            println!("cargo:warning=3. Running CMake to build C++ library...");
            let status = Command::new("cmake")
                .current_dir(&resonance_audio_dir)
                .arg("--build").arg("build")
                .arg("--config").arg("Release")
                .status();
            if let Ok(s) = status {
                if !s.success() {
                    panic!("CMake build failed. See output for details.");
                }
            } else {
                panic!("Failed to run CMake build step. Check your C++ compiler.");
            }
            println!("cargo:warning=✅ CMake build complete.");
        }
        Err(_) => {
            println!("cargo:warning=2a. CMake not found on PATH — skipping CMake configure/build.");
            println!("cargo:warning=If you don't have prebuilt ResonanceAudio artifacts, please install CMake or set VRAUDIO_LIB_DIR to point to a built library.");
        }
    }

    // --- Phase 3: Link the C++ library and build the cxx bridge ---
    println!("cargo:warning=4. Setting up linker and include paths for Rust build...");

    // Construct the library path explicitly based on the known structure
    let lib_name = env::var("VRAUDIO_LIB_NAME").unwrap_or_else(|_| "ResonanceAudioStatic".to_string());
    // Allow override of where the prebuilt resonance-audio library lives.
    let native_lib_dir = if let Ok(dir) = env::var("VRAUDIO_LIB_DIR") {
        PathBuf::from(dir)
    } else {
        build_dir.join("resonance_audio").join("Release")
    };
    let lib_path = native_lib_dir.join(format!("{}.lib", lib_name));

    // Verify the library exists before linking
    if !lib_path.exists() {
        panic!("Error: Could not find compiled C++ library at {}. Please check your CMake build output and build configuration (Debug vs Release).", lib_path.display());
    }

    println!("cargo:warning=   Found C++ library at: {}", native_lib_dir.display());
    println!("cargo:rustc-link-search=native={}", native_lib_dir.display());
    println!("cargo:rustc-link-lib=static={}", lib_name);

    println!("cargo:warning=5. Building Rust cxx bridge...");
    let mut build = cxx_build::bridge(&bridge_rs);
    build.file(&cpp_src_file);
    // Also compile the small wrapper that adapts vraudio types to the cxx-generated RA types.
    build.file("cxx/src/resonance_api_wrapper.cc");

    // Add all necessary include directories, as requested.
    // This is the crucial addition to find the generated cxx header.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // The cxx crate writes generated headers under: $OUT_DIR/cxxbridge/include/
    // Add that include dir so includes like "resonance-cxx/src/bridge.rs.h" resolve.
    build.include(&out_dir);
    build.include(out_dir.join("cxxbridge").join("include"));

    // Includes the local bridge header directory.
    build.include(&cpp_header_dir);

    // Corrected include path: Add the parent directory so the compiler can find 'resonance-audio-rs/src/api.h'
    let resonance_audio_rs_src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    if resonance_audio_rs_src_dir.exists() {
        build.include(&resonance_audio_rs_src_dir);
    } else {
        println!("cargo:warning=Skipping include for non-existent directory: {}", resonance_audio_rs_src_dir.display());
    }

    // Includes the main resonance-audio library headers.
    build.include(resonance_audio_dir.join("resonance_audio").join("api"));
    build.include(resonance_audio_dir.join("resonance_audio"));

    if let Ok(env_var) = env::var("VRAUDIO_INCLUDE") {
        println!("cargo:warning=   Using VRAUDIO_INCLUDE: {}", env_var);
        build.include(&env_var);
    }

    build.flag_if_supported("-std=c++17");
    build.compile("resonance_cxx_bridge");
    println!("cargo:warning=✅ Rust cxx bridge build complete.");

    println!("cargo:warning=--- Native C++ Build Process Finished ---");
}