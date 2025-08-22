use asset_manager::asset_pkg::AssetPkg;
use std::path::Path;

#[test]
fn pkg_lists_and_reads() {
    // Use the repo-local packaged assets produced by tools/pack-assets.ps1
    let pkg_path = Path::new("assets/dest/out.pkg");
    if !pkg_path.exists() {
        eprintln!("package not found at {:?}, skipping test", pkg_path);
        return;
    }

    let pkg = AssetPkg::open(pkg_path).expect("open pkg");
    let names: Vec<String> = pkg.list_names();
    assert!(!names.is_empty(), "expected at least one asset in package");

    // Read first entry bytes (should succeed)
    let first = &names[0];
    let bytes = pkg.read_asset_bytes_cow(first).expect("read asset");
    assert!(!bytes.is_empty(), "asset bytes non-empty");
}
