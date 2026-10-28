//! A cross-process lock protects active job directories from TTL reaping.
use super::*;
use std::fs::{self, File};

pub(super) struct Lease {
    // Fields drop in this order: close the lock before TempDir removes files on Windows.
    _lock: File,
    pub directory: tempfile::TempDir,
    _permit: OwnedSemaphorePermit,
}
impl Lease {
    pub fn new(root: &std::path::Path, permit: OwnedSemaphorePermit) -> std::io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("download-")
            .tempdir_in(root)?;
        let lock = File::create_new(directory.path().join(".lease"))?;
        lock.try_lock().map_err(std::io::Error::other)?;
        Ok(Self {
            _lock: lock,
            directory,
            _permit: permit,
        })
    }
}
pub(super) struct Cleanup(tokio::task::JoinHandle<()>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl Cleanup {
    pub fn start(settings: Arc<DownloadSettings>) -> Self {
        Self(tokio::spawn(async move {
            let mut timer =
                tokio::time::interval(Duration::from_secs(settings.cleanup_interval_seconds));
            loop {
                timer.tick().await;
                let settings = settings.clone();
                let result = tokio::task::spawn_blocking(move || {
                    reap(
                        &settings.temp_dir,
                        Duration::from_secs(settings.cleanup_ttl_seconds),
                    )
                })
                .await;
                if !matches!(result, Ok(Ok(()))) {
                    tracing::warn!("Download temporary-file cleanup failed");
                }
            }
        }))
    }
}
pub(super) fn reap(root: &std::path::Path, ttl: Duration) -> std::io::Result<()> {
    let root = fs::canonicalize(root)?;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)?;
        if !entry.file_name().to_string_lossy().starts_with("download-")
            || !meta.is_dir()
            || meta.is_symlink()
        {
            continue;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                continue;
            }
        }
        let canonical = fs::canonicalize(&path)?;
        if canonical.parent() != Some(root.as_path()) {
            continue;
        }
        let marker = canonical.join(".lease");
        let Ok(meta) = fs::symlink_metadata(&marker) else {
            continue;
        };
        if !meta.is_file()
            || meta.is_symlink()
            || meta.modified()?.elapsed().unwrap_or_default() < ttl
        {
            continue;
        }
        let lock = File::options().read(true).write(true).open(marker)?;
        if lock.try_lock().is_err() {
            continue;
        } // Active worker, body or another process.
        // No active owner can reuse this random job name. Close before deletion on Windows.
        drop(lock);
        match fs::remove_dir_all(canonical) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
