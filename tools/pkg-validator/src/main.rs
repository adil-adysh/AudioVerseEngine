use anyhow::{Context, Result};
use clap::Parser;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Parser)]
struct Args {
    /// Path to .pkg file
    path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate_pkg(&args.path)
}

fn validate_pkg(path: &str) -> Result<()> {
    use asset_manager::pkg_format::{AssetIndexEntry, PkgHeader};
    use bincode::Decode;

    // Open file
    let mut f = File::open(path).with_context(|| format!("opening {}", path))?;

    // Read header (bincode encoded). We'll read first 64KiB as a safe header read.
    let mut head_buf = vec![0u8; 64 * 1024];
    let n = f.read(&mut head_buf)?;
    head_buf.truncate(n);

    let header: PkgHeader = bincode::decode_from_slice(&head_buf, bincode::config::standard())
        .context("decoding PkgHeader")?
        .0;

    println!(
        "Header: magic=0x{:08x} version={} flags={} index_offset={} index_size={}",
        header.magic, header.version, header.flags, header.index_offset, header.index_size
    );

    // Read index bytes from file
    f.seek(SeekFrom::Start(header.index_offset))?;
    let mut index_bytes = vec![0u8; header.index_size as usize];
    f.read_exact(&mut index_bytes)?;

    // Verify index hash
    let mut hasher = Sha256::new();
    hasher.update(&index_bytes);
    let hash = hasher.finalize();
    if hash.as_slice() != header.index_hash {
        return Err(anyhow::anyhow!("index hash mismatch"));
    }

    // Decode a Vec<AssetIndexEntry>
    let (entries, _): (Vec<AssetIndexEntry>, _) =
        bincode::decode_from_slice(&index_bytes, bincode::config::standard())?;
    println!("Index entries: {}", entries.len());

    // Validate each entry's offsets and optional checksum
    let file_len = f.metadata()?.len();
    for e in entries.iter() {
        println!(
            "- {}: type={:?} offset={} size={}",
            e.name, e.asset_type, e.offset, e.size
        );
        if e.offset
            .checked_add(e.size)
            .map(|v| v > file_len)
            .unwrap_or(true)
        {
            return Err(anyhow::anyhow!(format!("entry '{}' out of range", e.name)));
        }
        if let Some(cs) = e.checksum {
            // compute sha256 on the entry bytes
            f.seek(SeekFrom::Start(e.offset))?;
            let mut blob = vec![0u8; e.size as usize];
            f.read_exact(&mut blob)?;
            let mut h = Sha256::new();
            h.update(&blob);
            let got = h.finalize();
            if got.as_slice() != cs {
                return Err(anyhow::anyhow!(format!(
                    "checksum mismatch for '{}'",
                    e.name
                )));
            }
        }
    }

    println!("pkg ok");
    Ok(())
}
