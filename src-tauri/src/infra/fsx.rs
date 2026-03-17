use std::path::{Path, PathBuf};

use atomicwrites::{AllowOverwrite, AtomicFile};
use time::{format_description::BorrowedFormatItem, OffsetDateTime};

use crate::error::{AppError, AppResult};

const BACKUP_FORMAT: &[BorrowedFormatItem<'_>] =
    time::macros::format_description!("[year][month][day]-[hour][minute][second]");

pub fn to_rel_string(root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(root) {
        rel.display().to_string().replace('\\', "/")
    } else {
        path.display().to_string().replace('\\', "/")
    }
}

pub fn backup_if_exists(path: &Path) -> AppResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let parent = path.parent().ok_or_else(|| {
        AppError::write_file_failed(path.display().to_string(), "无法获取父目录")
    })?;

    let ts = OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(BACKUP_FORMAT)
        .map_err(|e| AppError::write_file_failed(path.display().to_string(), e))?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("charge_plan.yml");
    let backup_name = file_name.replace(".yml", &format!(".{ts}.bak"));
    let backup_path = parent.join(backup_name);

    std::fs::copy(path, &backup_path)
        .map_err(|e| AppError::write_file_failed(backup_path.display().to_string(), e))?;

    Ok(Some(backup_path))
}

pub fn atomic_write_text(path: &Path, text: &str) -> AppResult<()> {
    let atomic = AtomicFile::new(path, AllowOverwrite);
    atomic
        .write(|f| -> std::io::Result<()> {
            use std::io::Write;
            f.write_all(text.as_bytes())?;
            if !text.ends_with('\n') {
                f.write_all(b"\n")?;
            }
            Ok(())
        })
        .map_err(|e: atomicwrites::Error<std::io::Error>| {
            AppError::write_file_failed(path.display().to_string(), e.to_string())
        })
}
