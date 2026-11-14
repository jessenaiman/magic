//! Crash-safe `craft-atomic` helpers for atomic file writes, exclusive creation,
//! private recovery files, and streaming atomic file copies into project directories.

use std::io::Read;
use std::path::{Path, PathBuf};

use craft_atomic::{NewFilePermissions, WriteOptions};
use serde::Serialize;

use crate::{EngineError, Result};

/// Maximum size buffered in memory when using `craft_atomic::write_new`; larger files stream via
/// `craft_atomic::atomic_write_with` after checking destination existence.
const EXCLUSIVE_BUFFER_LIMIT: u64 = 16 * 1024 * 1024;

pub use crate::link_watch::FileStamp;

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Write `value` to `path` as pretty JSON through [`craft_atomic::atomic_write`], creating the
/// folder, and return the file's new stamp and size.
pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(Option<FileStamp>, usize)> {
    let payload = serde_json::to_vec_pretty(value).map_err(|e| EngineError::InvalidParams(format!("cannot encode {}: {e}", path.display())))?;
    write_file(path, &payload)?;
    Ok((FileStamp::of(path), payload.len()))
}

/// Write `value` to `path` exclusively through [`craft_atomic::write_new`], refusing with
/// `AlreadyExists` without modifying any existing file on disk.
pub(crate) fn write_json_new(path: &Path, value: &impl Serialize) -> Result<(Option<FileStamp>, usize)> {
    let payload = serde_json::to_vec_pretty(value).map_err(|e| EngineError::InvalidParams(format!("cannot encode {}: {e}", path.display())))?;
    write_file_new(path, &payload)?;
    Ok((FileStamp::of(path), payload.len()))
}

/// Write private recovery JSON (`NewFilePermissions::Private`, `0600` on Unix) atomically.
pub(crate) fn write_private_json(path: &Path, value: &impl Serialize) -> Result<(Option<FileStamp>, usize)> {
    let payload = serde_json::to_vec_pretty(value).map_err(|e| EngineError::InvalidParams(format!("cannot encode {}: {e}", path.display())))?;
    write_private_file(path, &payload)?;
    Ok((FileStamp::of(path), payload.len()))
}

/// Write `bytes` to `path` through [`craft_atomic::atomic_write`], creating the folder.
pub fn write_file(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    craft_atomic::atomic_write(path, bytes.as_ref())?;
    Ok(())
}

/// Create `path` exclusively through [`craft_atomic::write_new`] (`O_EXCL` / hard-link staging),
/// failing with `AlreadyExists` rather than replacing an existing document.
pub fn write_file_new(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    craft_atomic::write_new(path, bytes.as_ref())?;
    Ok(())
}

/// Atomically write private data staged up front with [`NewFilePermissions::Private`] (`0600` on Unix).
pub fn write_private_file(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private).atomic_write(path, bytes.as_ref())?;
    Ok(())
}

/// Exclusively create private data (`NewFilePermissions::Private` + `write_new`), failing with
/// `AlreadyExists` without touching any existing file.
pub fn write_private_file_new(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    WriteOptions::default().with_new_file_permissions(NewFilePermissions::Private).write_new(path, bytes.as_ref())?;
    Ok(())
}

fn files_have_identical_bytes(a: &Path, b: &Path) -> Result<bool> {
    let mut fa = crate::read_only_file::open_regular(a)?;
    let mut fb = crate::read_only_file::open_regular(b)?;
    if fa.metadata()?.len() != fb.metadata()?.len() {
        return Ok(false);
    }
    let mut buf_a = vec![0_u8; 64 * 1024];
    let mut buf_b = vec![0_u8; 64 * 1024];
    loop {
        let na = fa.read(&mut buf_a)?;
        if na == 0 {
            return Ok(true);
        }
        fb.read_exact(&mut buf_b[..na])?;
        if buf_a[..na] != buf_b[..na] {
            return Ok(false);
        }
    }
}

/// Crash-safe atomic copy of `source` into `dest` using `craft-atomic` (`write_new` for exclusive
/// creation up to 16 MiB and streaming `atomic_write_with` for large media or replacements).
pub(crate) fn atomic_copy_file(source: &Path, dest: &Path, overwrite: bool) -> Result<u64> {
    if source == dest {
        return Ok(std::fs::metadata(dest)?.len());
    }
    if let (Ok(canon_src), Ok(canon_dst)) = (source.canonicalize(), dest.canonicalize())
        && canon_src == canon_dst
    {
        return Ok(std::fs::metadata(&canon_dst)?.len());
    }
    let mut reader = crate::read_only_file::open_regular(source)?;
    let len = reader.metadata()?.len();
    ensure_parent(dest)?;

    if !overwrite && dest.exists() {
        if dest.is_file() && files_have_identical_bytes(source, dest).unwrap_or(false) {
            return Ok(len);
        }
        return Err(EngineError::InvalidParams(format!(
            "destination already exists with different bytes: {} (pass overwrite=true or rename to avoid collision)",
            dest.display()
        )));
    }

    if !overwrite && len <= EXCLUSIVE_BUFFER_LIMIT {
        let mut buf = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
        reader.read_to_end(&mut buf)?;
        match craft_atomic::write_new(dest, &buf) {
            Ok(()) => return Ok(len),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if dest.is_file() && files_have_identical_bytes(source, dest).unwrap_or(false) {
                    return Ok(len);
                }
                return Err(EngineError::InvalidParams(format!("destination already exists with different bytes: {}", dest.display())));
            }
            Err(e) => return Err(e.into()),
        }
    }

    craft_atomic::atomic_write_with(dest, |out| std::io::copy(&mut reader, out).map(|_| ()))?;
    Ok(len)
}

/// Recursively copy a Craft package directory bundle into `dest_dir`, writing every regular file
/// atomically via [`atomic_copy_file`].
pub(crate) fn atomic_copy_dir(source_dir: &Path, dest_dir: &Path, overwrite: bool) -> Result<usize> {
    if let (Ok(cs), Ok(cd)) = (source_dir.canonicalize(), dest_dir.canonicalize())
        && cs == cd
    {
        return Ok(0);
    }
    std::fs::create_dir_all(dest_dir)?;
    let mut count = 0usize;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(source_dir)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        let Some(name) = entry.file_name() else { continue };
        if name.to_str().is_some_and(|n| n.starts_with('.') && n.ends_with(".craft_tmp")) {
            continue;
        }
        let dst = dest_dir.join(name);
        let meta = std::fs::symlink_metadata(&entry)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            count += atomic_copy_dir(&entry, &dst, overwrite)?;
        } else if meta.is_file() {
            atomic_copy_file(&entry, &dst, overwrite)?;
            count += 1;
        }
    }
    Ok(count)
}
