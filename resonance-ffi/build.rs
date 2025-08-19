use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../resonance-audio/resonance_audio/api/resonance_audio_api.h");
    let msvc_include = "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/include";
    let windows_sdk_include = "C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0/ucrt";
    let llvm_include = "C:/Program Files/LLVM/include";

    let bindings = bindgen::Builder::default()
        .header("../resonance-audio/resonance_audio/api/resonance_audio_api.h")
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++14")
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

    // Patch extern "C" blocks to unsafe extern "C"
    use std::fs;
    let content = fs::read_to_string(&bindings_path).expect("Couldn't read generated bindings");
    let patched = content.replace("extern \"C\"", "unsafe extern \"C\"");
    fs::write(&bindings_path, patched).expect("Couldn't patch bindings");
}
