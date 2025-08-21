use asset_manager::pkg_format::{PkgHeader, AssetIndexEntry, AssetType};
use asset_manager::asset_pkg::AssetPkg;
use bincode::{config, encode_to_vec};
use std::io::Write;

#[test]
fn pkg_roundtrip() {
    // make two fake assets
    let a1 = b"HELLO".to_vec();
    let a2 = b"WORLD!".to_vec();
    // build index with offsets relative to the start of the assets block (0-based)
    let mut base_entries = Vec::new();
    let mut rel_offset = 0u64;
    base_entries.push(AssetIndexEntry {
        name: "foo.sfx".to_string(),
        asset_type: AssetType::Sfx,
        offset: rel_offset,
        size: a1.len() as u64,
        sample_rate: 48000,
        channels: 2,
        flags: 0,
        checksum: None,
    });
    rel_offset += a1.len() as u64;
    base_entries.push(AssetIndexEntry {
        name: "bar.wav".to_string(),
        asset_type: AssetType::Music,
        offset: rel_offset,
        size: a2.len() as u64,
        sample_rate: 48000,
        channels: 2,
        flags: 0,
        checksum: None,
    });

    let config = config::standard();

    // We'll iterate until the header encoding size stabilizes so absolute offsets are correct.
    // Keep a copy of the base (relative) entries so we don't repeatedly add header_len to already-absolute offsets.
    // (base_entries is already the relative entries)
    let mut hdr_bytes = Vec::new();
    let mut final_entries: Vec<AssetIndexEntry> = Vec::new();
    for _ in 0..8 {
        // compute index bytes from base entries (relative offsets)
        let mut index_bytes = encode_to_vec(&base_entries, config).expect("encode index");

        // provisional header with index_offset = 0 to get a header-length estimate
        let provisional = PkgHeader::new(0, &index_bytes, 0);
        let provisional_hdr = encode_to_vec(&provisional, config).expect("encode hdr");
        let header_len = provisional_hdr.len() as u64;

        // compute absolute offsets for entries from base (relative) entries: header_len + relative offset
        let mut abs_entries = base_entries.clone();
        for e in abs_entries.iter_mut() {
            e.offset += header_len;
        }

    // re-encode index with absolute offsets
    index_bytes = encode_to_vec(&abs_entries, config).expect("encode index abs");

        let index_offset = header_len + a1.len() as u64 + a2.len() as u64;
        let hdr = PkgHeader::new(index_offset, &index_bytes, 0);
        let new_hdr_bytes = encode_to_vec(&hdr, config).expect("encode final hdr");

        if !hdr_bytes.is_empty() && hdr_bytes.len() == new_hdr_bytes.len() {
            // header stabilized
            hdr_bytes = new_hdr_bytes;
            // keep abs_entries for final assignment outside the loop
            final_entries = abs_entries;
            break;
        }

    hdr_bytes = new_hdr_bytes;
    // store latest abs_entries for final write
    final_entries = abs_entries;
    }

    // write final file: header, assets, index
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(&hdr_bytes).unwrap();
    tmp.write_all(&a1).unwrap();
    tmp.write_all(&a2).unwrap();
    // ensure final index bytes match final_entries
    let index_bytes = encode_to_vec(&final_entries, config).expect("encode final index");
    tmp.write_all(&index_bytes).unwrap();
    tmp.flush().unwrap();

    // open with AssetPkg
    let pkg = AssetPkg::open(tmp.path()).expect("open pkg");
    let foo = pkg.read_asset_bytes("foo.sfx").expect("read foo");
    assert_eq!(foo, a1);
    let bar = pkg.read_asset_bytes("bar.wav").expect("read bar");
    assert_eq!(bar, a2);
}
