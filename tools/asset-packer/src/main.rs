use asset_manager::pkg_format::{AssetIndexEntry, AssetType, PkgHeader};
use bincode::config::standard;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Smarter packer:
// - Accepts files and directories; directories are scanned recursively
// - Computes per-asset SHA-256 checksum and probes sample_rate/channels for audio using symphonia
// - Usage examples:
//   cargo run -p asset-packer -- out.pkg path/to/dir other/file.sfx

fn probe_audio_metadata(path: &Path) -> Option<(u32, u16)> {
    // Use symphonia to probe sample rate and channels for supported audio files
    if let Ok(file) = std::fs::File::open(path) {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::default::get_probe;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        if let Ok(probed) = get_probe().format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            if let Some(track) = probed.format.default_track() {
                if let Some(params) = track.codec_params.sample_rate {
                    let channels =
                        track.codec_params.channels.map(|c| c.count()).unwrap_or(0) as u16;
                    return Some((params, channels));
                }
            }
        }
    }
    None
}

fn compute_checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut h = [0u8; 32];
    h.copy_from_slice(&out);
    h
}

fn collect_inputs(args: &[String], recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in args.iter() {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            if recursive {
                for entry in WalkDir::new(&pb)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            } else if let Ok(rd) = std::fs::read_dir(&pb) {
                for e in rd.filter_map(|r| r.ok()) {
                    let p = e.path();
                    if p.is_file() {
                        files.push(p);
                    }
                }
            }
        } else if pb.is_file() {
            files.push(pb);
        }
    }
    files
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: asset-packer [-r|--recursive] out.pkg path_or_file [path_or_file ...]");
        std::process::exit(1);
    }

    // simple flag parsing: accept -r/--recursive and --no-recursive before the out file
    let mut recursive = false;
    let mut idx = 1;
    while idx < args.len() && args[idx].starts_with('-') {
        match args[idx].as_str() {
            "-r" | "--recursive" => recursive = true,
            "--no-recursive" => recursive = false,
            _ => {
                eprintln!("unknown option: {}", args[idx]);
                std::process::exit(1);
            }
        }
        idx += 1;
    }

    if idx >= args.len() {
        eprintln!("missing output file");
        std::process::exit(1);
    }

    let out = Path::new(&args[idx]);
    idx += 1;
    if idx >= args.len() {
        eprintln!("no input paths provided");
        std::process::exit(1);
    }

    let inputs = collect_inputs(&args[idx..], recursive);
    if inputs.is_empty() {
        eprintln!("no input files found");
        std::process::exit(1);
    }

    // open output and write placeholder header
    let mut f = File::create(out)?;
    let header_placeholder = vec![0u8; 512];
    f.write_all(&header_placeholder)?;

    // gather entries
    let mut entries: Vec<AssetIndexEntry> = Vec::new();
    for path in inputs.iter() {
        let data = std::fs::read(path)?;
        let offset = f.stream_position()?;
        f.write_all(&data)?;
        let size = data.len() as u64;
        let checksum = compute_checksum(&data);

        let asset_type = match path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("sfx") => AssetType::Sfx,
            Some("wav") | Some("ogg") | Some("mp3") => AssetType::Music,
            _ => AssetType::Other,
        };

        let mut sample_rate = 0u32;
        let mut channels = 0u16;
        if let Some((sr, ch)) = probe_audio_metadata(path) {
            sample_rate = sr;
            channels = ch;
        }

        // use relative path as asset name when possible
        let name = path.to_string_lossy().into_owned();

        entries.push(AssetIndexEntry {
            name,
            asset_type,
            offset,
            size,
            sample_rate,
            channels,
            flags: 0,
            checksum: Some(checksum),
        });
    }

    let index_bytes = bincode::encode_to_vec(&entries, standard())?;
    let index_offset = f.stream_position()?;
    f.write_all(&index_bytes)?;

    let header = PkgHeader::new(index_offset, &index_bytes, 0);
    let header_bytes = bincode::encode_to_vec(&header, standard())?;

    // rewrite header
    f.seek(SeekFrom::Start(0))?;
    if header_bytes.len() > header_placeholder.len() {
        return Err("header too large for placeholder".into());
    }
    f.write_all(&header_bytes)?;
    let pad = header_placeholder.len() - header_bytes.len();
    if pad > 0 {
        f.write_all(&vec![0u8; pad])?;
    }

    println!("wrote {} with {} assets", out.display(), entries.len());
    Ok(())
}
