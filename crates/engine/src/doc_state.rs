//! One document of the open project, held by the session (`docs/architecture.md` › Engine shape),
//! plus `craft-atomic` helpers for exclusive creation, private recovery/token files, and streaming
//! atomic file copies into project directories.
//!
//! Views, the CLI, MCP and the control channel read and edit this copy through commands; the file is
//! written only by [`DocState::save`]. The file's stamp at load time lets the session notice a change
//! made by another app or agent (see [`crate::link_watch`]): an unedited document reloads, an edited
//! one refuses to save over the newer file.

use std::io::Read;
use std::path::{Path, PathBuf};

use craft_atomic::{NewFilePermissions, WriteOptions};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{EngineError, Result};

/// Maximum size buffered in memory when using `craft_atomic::write_new`; larger files stream via
/// `craft_atomic::atomic_write_with` after checking destination existence.
const EXCLUSIVE_BUFFER_LIMIT: u64 = 16 * 1024 * 1024;

/// A file Magic owns in a project folder, read and written only through the session
/// ([`crate::Session::doc`], [`crate::Session::save_doc`]): atomic, guarded against outside
/// changes by its [`FileStamp`], and undoable. A new kind of project file is one type with this
/// impl; no view or command writes the file itself.
pub trait ProjectDoc: Serialize + DeserializeOwned {
    /// The session's name for this kind of document.
    const KIND: &'static str;
    /// Where the document lives in the project folder `dir`.
    fn path(dir: &Path) -> PathBuf;
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Write `value` to `path` as pretty JSON through [`craft_atomic::atomic_write`], creating the
/// folder, and return the file's new stamp and size. The one JSON writer for Magic's files:
/// callers apply their own outside-change guard first.
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

/// Write private recovery/journal JSON (`NewFilePermissions::Private`, `0600` on Unix) atomically.
pub(crate) fn write_private_json(path: &Path, value: &impl Serialize) -> Result<(Option<FileStamp>, usize)> {
    let payload = serde_json::to_vec_pretty(value).map_err(|e| EngineError::InvalidParams(format!("cannot encode {}: {e}", path.display())))?;
    write_private_file(path, &payload)?;
    Ok((FileStamp::of(path), payload.len()))
}

/// Write `bytes` to `path` through [`craft_atomic::atomic_write`], creating the folder. The one
/// file writer for Magic (`std::fs::write` is disallowed in `clippy.toml`): a reader never sees a
/// half-written file, whatever the content.
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

/// Atomically write private data (`.autosave` recovery journals, `.history/*.bak` snapshots, tokens)
/// staged up front with [`NewFilePermissions::Private`] (`0600` on Unix).
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
/// Never truncates `dest` in place and never leaves a half-copied file on failure.
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

/// Recursively copy a Craft package directory bundle (such as a directory-mode `.pcraft`) into
/// `dest_dir`, writing every regular file atomically via [`atomic_copy_file`].
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

pub use crate::link_watch::FileStamp;

#[derive(Clone, Debug)]
pub struct DocState {
    /// What the document is, e.g. `words` for the lyric design (`design/words.json`).
    pub kind: &'static str,
    pub path: PathBuf,
    pub value: Value,
    /// Bumped by every edit, so readers can tell whether they hold the latest copy.
    pub revision: u64,
    /// Edited since it was loaded or saved.
    pub dirty: bool,
    /// The file as it was when the document last matched it (`None`: no file yet).
    pub stamp: Option<crate::link_watch::FileStamp>,
}

impl DocState {
    /// Read `path` as JSON. A missing file is `Ok(None)`.
    pub fn load(kind: &'static str, path: &Path) -> Result<Option<Self>> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let value = serde_json::from_slice(&bytes).map_err(|e| EngineError::InvalidParams(format!("{} is not valid JSON: {e}", path.display())))?;
        Ok(Some(Self { kind, path: path.to_path_buf(), value, revision: 0, dirty: false, stamp: crate::link_watch::FileStamp::of(path) }))
    }

    /// A document whose file does not exist yet.
    pub fn new(kind: &'static str, path: &Path, value: Value) -> Self {
        Self { kind, path: path.to_path_buf(), value, revision: 0, dirty: true, stamp: None }
    }

    /// Whether another writer changed the file since the document last matched it.
    pub fn changed_on_disk(&self) -> bool {
        crate::link_watch::changed_on_disk(&self.path, self.stamp)
    }

    /// Replace the document's content as one edit.
    pub fn set(&mut self, value: Value) {
        self.value = value;
        self.revision += 1;
        self.dirty = true;
    }

    /// Write the document atomically. Refuses when another writer changed the file since the
    /// document last matched it, so their work is never silently overwritten.
    pub fn save(&mut self) -> Result<()> {
        crate::link_watch::save_guard(&self.path, self.stamp)?;
        self.stamp = write_json(&self.path, &self.value)?.0;
        self.dirty = false;
        Ok(())
    }
}
