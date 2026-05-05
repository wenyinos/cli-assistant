//! File utilities.
//!
//! Maps to Python `utils/files.py`. Provides folder creation, file writing with
//! restricted permissions, MIME type guessing, and advisory file locking.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::marker::PhantomData;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::environment::get_xdg_state_path;
use crate::errors::{ClaError, Result};

// ---------------------------------------------------------------------------
// Directory & file helpers
// ---------------------------------------------------------------------------

/// Creates a directory at `path` with the given Unix permission mode.
///
/// If `parents` is `true`, intermediate directories are created as needed
/// (like `mkdir -p`). Existing directories are silently ignored.
pub fn create_folder(path: &Path, parents: bool, mode: u32) -> Result<()> {
    let result = if parents {
        std::fs::create_dir_all(path)
    } else {
        std::fs::create_dir(path)
    };

    match result {
        Ok(()) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => {
            return Err(ClaError::io(Some(path.to_path_buf()), e));
        }
    }

    set_permissions(path, mode)?;
    tracing::debug!("Created directory {:?} with mode {:o}", path, mode);
    Ok(())
}

/// Writes `contents` to the file at `path` with the given Unix permission mode.
///
/// If the file already exists it is overwritten. Parent directories are **not**
/// created automatically.
pub fn write_file(contents: &[u8], path: &Path, mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)
        .map_err(|e| ClaError::io(Some(path.to_path_buf()), e))?;

    file.write_all(contents)
        .map_err(|e| ClaError::io(Some(path.to_path_buf()), e))?;

    // Ensure permissions are set even if the file already existed
    set_permissions(path, mode)?;
    tracing::debug!("Wrote {} bytes to {:?}", contents.len(), path);
    Ok(())
}

/// Guesses the MIME type for the given file path.
///
/// Returns `"application/octet-stream"` if the type cannot be determined.
pub fn guess_mimetype(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string()
}

/// Sets file/directory permissions using a Unix mode bitmask.
fn set_permissions(path: &Path, mode: u32) -> Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| ClaError::io(Some(path.to_path_buf()), e))
}

// ---------------------------------------------------------------------------
// NamedFileLock
// ---------------------------------------------------------------------------

/// Advisory file lock identified by a name.
///
/// Lock files are stored under `<XDG_STATE_HOME>/locks/<name>.lock` and contain
/// the PID of the holding process. Uses `fs2` (POSIX `flock`) for locking.
///
/// The lock is automatically released when the value is dropped.
pub struct NamedFileLock {
    file: File,
    path: PathBuf,
    // `flock` is per-fd and not thread-safe across different handles.
    // PhantomData<*const ()> makes this type !Sync while remaining Send.
    _not_sync: PhantomData<*const ()>,
}

impl NamedFileLock {
    /// Creates (but does not acquire) a lock for the given `name`.
    ///
    /// The lock file is created under the XDG state directory. The `name` is
    /// sanitized to replace path separators with underscores.
    pub fn new(name: &str) -> Result<Self> {
        let safe_name = name.replace('/', "_").replace('\\', "_");
        let lock_dir = get_xdg_state_path().join("locks");
        create_folder(&lock_dir, true, 0o700)?;

        let path = lock_dir.join(format!("{}.lock", safe_name));
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| ClaError::io(Some(path.clone()), e))?;

        Ok(Self {
            file,
            path,
            _not_sync: PhantomData,
        })
    }

    /// Returns `true` if the lock is currently held (by any process).
    ///
    /// This checks whether an exclusive lock can be acquired without blocking;
    /// if it cannot, the lock is held.
    pub fn is_locked(&self) -> bool {
        self.file.try_lock_exclusive().is_err()
    }

    /// Acquires an exclusive lock, blocking until it is available.
    ///
    /// After acquiring the lock, the current process PID is written to the lock
    /// file for diagnostics.
    pub fn acquire(&self) -> Result<()> {
        self.file.lock_exclusive().map_err(|e| {
            ClaError::io(Some(self.path.clone()), e)
        })?;

        // Write PID for diagnostics
        let pid = std::process::id().to_string();
        // Truncate and write (we already have the lock, so this is safe)
        let _ = std::fs::write(&self.path, &pid);

        tracing::debug!("Acquired lock {:?}", self.path);
        Ok(())
    }

    /// Releases the lock and clears the PID from the lock file.
    pub fn release(&self) -> Result<()> {
        let _ = std::fs::write(&self.path, b"");
        self.file.unlock().map_err(|e| {
            ClaError::io(Some(self.path.clone()), e)
        })?;
        tracing::debug!("Released lock {:?}", self.path);
        Ok(())
    }

    /// Returns the path to the lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for NamedFileLock {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_folder_and_write_file() {
        let dir = std::env::temp_dir().join("cla_common_test_files");
        let _ = fs::remove_dir_all(&dir);

        create_folder(&dir, true, 0o755).unwrap();
        assert!(dir.is_dir());

        let file_path = dir.join("test.txt");
        write_file(b"hello world", &file_path, 0o644).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "hello world");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn guess_mimetype_known_types() {
        assert_eq!(
            guess_mimetype(Path::new("image.png")),
            "image/png"
        );
        assert_eq!(
            guess_mimetype(Path::new("readme.txt")),
            "text/plain"
        );
        assert_eq!(
            guess_mimetype(Path::new("data.json")),
            "application/json"
        );
    }

    #[test]
    fn guess_mimetype_unknown_defaults_to_octet_stream() {
        assert_eq!(
            guess_mimetype(Path::new("file.unknownext123")),
            "application/octet-stream"
        );
    }

    #[test]
    fn named_file_lock_acquire_release() {
        let lock = NamedFileLock::new("cla_test_lock").unwrap();
        assert!(!lock.is_locked());

        lock.acquire().unwrap();
        assert!(lock.is_locked());

        lock.release().unwrap();
        assert!(!lock.is_locked());
    }

    #[test]
    fn named_file_lock_drops_cleanly() {
        let lock = NamedFileLock::new("cla_test_drop").unwrap();
        lock.acquire().unwrap();
        drop(lock);
        // After drop, a new lock on the same name should succeed
        let lock2 = NamedFileLock::new("cla_test_drop").unwrap();
        lock2.acquire().unwrap();
        lock2.release().unwrap();
    }

    #[test]
    fn create_folder_existing_is_ok() {
        let dir = std::env::temp_dir().join("cla_common_test_existing");
        let _ = fs::remove_dir_all(&dir);

        create_folder(&dir, true, 0o755).unwrap();
        // Calling again should succeed silently
        create_folder(&dir, true, 0o755).unwrap();

        let _ = fs::remove_dir_all(&dir);
    }
}
