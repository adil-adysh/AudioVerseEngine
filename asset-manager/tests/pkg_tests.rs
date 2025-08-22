// This is a test file for AssetPkg
use asset_manager::asset_pkg::AssetPkg;
use asset_manager::pkg_format::AssetIndexEntry;
use asset_manager::pkg_format::AssetType;
use asset_manager::pkg_format::PkgHeader;
use bincode::config::standard;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn make_pkg_with_one_asset(name: &str, data: &[u8]) -> Vec<u8> {
    // Use the same header stabilization approach as pkg_roundtrip: write header, asset(s), index.
    use bincode::encode_to_vec;
    let config = standard();
    // base (relative) entry
    let base_entries = vec![AssetIndexEntry {
        name: name.to_string(),
        asset_type: AssetType::Other,
        offset: 0u64,
        size: data.len() as u64,
        sample_rate: 0,
        channels: 0,
        flags: 0,
        checksum: None,
    }];

    let mut hdr_bytes = Vec::new();
    let mut final_entries: Vec<AssetIndexEntry> = Vec::new();
    for _ in 0..8 {
        let mut index_bytes = encode_to_vec(&base_entries, config).expect("encode index");

        let provisional = PkgHeader::new(0, &index_bytes, 0);
        let provisional_hdr = encode_to_vec(&provisional, config).expect("encode hdr");
        let header_len = provisional_hdr.len() as u64;

        let mut abs_entries = base_entries.clone();
        for e in abs_entries.iter_mut() {
            e.offset += header_len; // assets written immediately after header
        }

        index_bytes = encode_to_vec(&abs_entries, config).expect("encode index abs");
        let index_offset = header_len + data.len() as u64; // assets placed after header
        let hdr = PkgHeader::new(index_offset, &index_bytes, 0);
        let new_hdr_bytes = encode_to_vec(&hdr, config).expect("encode final hdr");

        if !hdr_bytes.is_empty() && hdr_bytes.len() == new_hdr_bytes.len() {
            hdr_bytes = new_hdr_bytes;
            final_entries = abs_entries;
            break;
        }

        hdr_bytes = new_hdr_bytes;
        final_entries = abs_entries;
    }

    // assemble file: header, asset, index
    let mut out = Vec::new();
    out.extend_from_slice(&hdr_bytes);
    out.extend_from_slice(data);
    let index_bytes = encode_to_vec(&final_entries, config).expect("encode final index");
    out.extend_from_slice(&index_bytes);
    out
}

#[test]
fn assetpkg_open_and_read_asset() {
    let dir = tempdir().unwrap();
    let data = b"hello world";
    let pkg = make_pkg_with_one_asset("a", data);
    let p = dir.path().join("p.pkg");
    let mut f = File::create(&p).unwrap();
    f.write_all(&pkg).unwrap();
    let ap = AssetPkg::open(&p).unwrap();
    let names = ap.list_names();
    assert_eq!(names, vec!["a".to_string()]);
    let got = ap.read_asset_bytes("a").unwrap();
    if got.as_slice() != data {
        eprintln!("got: {:?}", got);
        eprintln!("expected: {:?}", data);
    }
    assert_eq!(&got[..], &data[..]);
}

#[test]
fn assetpkg_index_hash_mismatch_fails() {
    let dir = tempdir().unwrap();
    let data = b"x";
    let mut pkg = make_pkg_with_one_asset("k", data);
    // corrupt one byte in index area
    // attempt to locate index start: header is encoded into first bytes; index follows asset
    // naive guess: index starts after header + asset; find occurrence of our data to locate asset position
    let asset_pos = pkg.windows(data.len()).position(|w| w == data).unwrap();
    eprintln!("asset_pos={}, pkg_len={}", asset_pos, pkg.len());
    // index starts after asset_pos + data.len()
    let idx = asset_pos + data.len();
    eprintln!("corrupting index at {}", idx);
    pkg[idx] ^= 0xff;
    let p = dir.path().join("bad.pkg");
    let mut f = File::create(&p).unwrap();
    f.write_all(&pkg).unwrap();
    let res = AssetPkg::open(&p);
    assert!(res.is_err());
}
