//! Pure-Rust `.tar.gz` archiver (`flate2` + POSIX `ustar` headers + `sha2`) that preserves and
//! compresses converted legacy projects inside `<project_dir>/magic/originals/` tagged `"legacy"`.

use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{EngineError, Result};

const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", "dist", "build", ".next", ".turbo", ".cache", "__pycache__"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyArchiveReceipt {
    pub status: String,
    pub archive_rel: String,
    pub manifest_rel: String,
    pub archive_path: PathBuf,
    pub manifest_path: PathBuf,
    pub file_count: usize,
    pub raw_bytes: u64,
    pub compressed_bytes: u64,
    pub sha256: String,
}

fn should_skip_entry(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if name.ends_with(".craft_tmp") || name == ".DS_Store" {
        return true;
    }
    SKIP_DIRS.contains(&name)
}

fn collect_source_files(root: &Path, cur: &Path, out: &mut Vec<(String, PathBuf, u64)>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(cur)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        if should_skip_entry(&entry) {
            continue;
        }
        if entry.is_dir() {
            if entry.join("originals").is_dir() && entry.file_name().and_then(|n| n.to_str()) == Some("magic") {
                continue;
            }
            collect_source_files(root, &entry, out)?;
        } else if entry.is_file() {
            let len = std::fs::metadata(&entry).map(|m| m.len()).unwrap_or(0);
            let rel = entry.strip_prefix(root).unwrap_or(&entry).to_string_lossy().replace('\\', "/");
            out.push((rel, entry, len));
        }
    }
    Ok(())
}

fn write_octal(buf: &mut [u8], val: u64) {
    let width = buf.len().saturating_sub(1);
    let formatted = format!("{val:0width$o}");
    let bytes = formatted.as_bytes();
    let start = bytes.len().saturating_sub(width);
    let slice = &bytes[start..];
    buf[..slice.len()].copy_from_slice(slice);
    if let Some(last) = buf.last_mut() {
        *last = 0;
    }
}

fn split_ustar_path(rel_path: &str) -> (String, String) {
    if rel_path.len() <= 100 {
        return (String::new(), rel_path.to_owned());
    }
    if let Some(pos) = rel_path.rfind('/') {
        let prefix = &rel_path[..pos];
        let name = &rel_path[pos + 1..];
        if prefix.len() <= 155 && name.len() <= 100 {
            return (prefix.to_owned(), name.to_owned());
        }
    }
    let tail_start = rel_path.len().saturating_sub(99);
    (String::new(), rel_path[tail_start..].to_owned())
}

fn build_ustar_header(rel_path: &str, size: u64, mtime: u64) -> [u8; 512] {
    let mut hdr = [0_u8; 512];
    let (prefix, name) = split_ustar_path(rel_path);
    let name_bytes = name.as_bytes();
    hdr[..name_bytes.len().min(100)].copy_from_slice(&name_bytes[..name_bytes.len().min(100)]);
    write_octal(&mut hdr[100..108], 0o644);
    write_octal(&mut hdr[108..116], 1000);
    write_octal(&mut hdr[116..124], 1000);
    write_octal(&mut hdr[124..136], size);
    write_octal(&mut hdr[136..148], mtime);
    hdr[148..156].fill(b' ');
    hdr[156] = b'0';
    hdr[257..263].copy_from_slice(b"ustar\0");
    hdr[263..265].copy_from_slice(b"00");
    if !prefix.is_empty() {
        let pbytes = prefix.as_bytes();
        hdr[345..345 + pbytes.len().min(155)].copy_from_slice(&pbytes[..pbytes.len().min(155)]);
    }
    let checksum: u32 = hdr.iter().map(|&b| u32::from(b)).sum();
    let chk_str = format!("{checksum:06o}\0 ");
    hdr[148..156].copy_from_slice(chk_str.as_bytes());
    hdr
}

/// Archive and gzip-compress a legacy source project directory into
/// `<project_dir>/magic/originals/<slug>-legacy.tar.gz` alongside `legacy-manifest.json`.
pub fn archive_legacy_project(source_dir: &Path, project_dir: &Path, slug: &str) -> Result<LegacyArchiveReceipt> {
    if !source_dir.is_dir() {
        return Err(EngineError::InvalidParams(format!("legacy source directory does not exist: {}", source_dir.display())));
    }
    let originals_dir = project_dir.join("magic/originals");
    std::fs::create_dir_all(&originals_dir)?;

    let mut files = Vec::new();
    collect_source_files(source_dir, source_dir, &mut files)?;

    let archive_name = format!("{slug}-legacy.tar.gz");
    let archive_rel = format!("magic/originals/{archive_name}");
    let manifest_rel = "magic/originals/legacy-manifest.json".to_owned();
    let archive_path = originals_dir.join(&archive_name);
    let manifest_path = originals_dir.join("legacy-manifest.json");

    let mut gz_buf = Vec::new();
    let mut raw_bytes = 0_u64;
    let mut manifest_entries: Vec<Value> = Vec::with_capacity(files.len());

    {
        let mut encoder = GzEncoder::new(&mut gz_buf, Compression::default());
        for (rel, abs, len) in &files {
            let data = std::fs::read(abs)?;
            let actual_len = data.len() as u64;
            raw_bytes = raw_bytes.saturating_add(actual_len);
            let mtime = std::fs::metadata(abs)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            let tar_rel = format!("{slug}-legacy/{rel}");
            let hdr = build_ustar_header(&tar_rel, actual_len, mtime);
            encoder.write_all(&hdr)?;
            encoder.write_all(&data)?;
            let pad = (512 - (actual_len % 512)) % 512;
            if pad > 0 {
                let zeros = [0_u8; 512];
                encoder.write_all(&zeros[..pad as usize])?;
            }
            manifest_entries.push(json!({
                "path": rel,
                "bytes": len,
                "massive_compressed": *len >= 10 * 1024 * 1024,
            }));
        }
        encoder.write_all(&[0_u8; 1024])?;
        encoder.finish()?;
    }

    let mut hasher = Sha256::new();
    hasher.update(&gz_buf);
    let sha_hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let compressed_bytes = gz_buf.len() as u64;

    let _ = crate::doc_state::write_file(&archive_path, &gz_buf)?;
    let manifest_doc = json!({
        "format": "magic.legacy_archive",
        "version": 1,
        "tag": "legacy",
        "status": "archived",
        "source_dir": source_dir,
        "archive": archive_name,
        "sha256": sha_hex,
        "file_count": files.len(),
        "raw_bytes": raw_bytes,
        "compressed_bytes": compressed_bytes,
        "files": manifest_entries,
    });
    let _ = crate::doc_state::write_json(&manifest_path, &manifest_doc)?;

    Ok(LegacyArchiveReceipt {
        status: "legacy".into(),
        archive_rel,
        manifest_rel,
        archive_path,
        manifest_path,
        file_count: files.len(),
        raw_bytes,
        compressed_bytes,
        sha256: sha_hex,
    })
}
