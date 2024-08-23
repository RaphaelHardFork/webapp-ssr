use crate::{Error, Result};
use std::{
    fs::{self, File},
    path::Path,
};

// region:        --- Directories

/// Returns true if one or more dir was created
pub fn ensure_dir(dir: &Path) -> Result<bool> {
    if dir.is_dir() {
        Ok(false)
    } else {
        fs::create_dir_all(dir).map_err(|ex| Error::CannotCreateDir(ex.to_string()))?;
        Ok(true)
    }
}

// endregion:     --- Directories

// region:        --- Files

/// Returns true if one or more dir/file was created
pub fn create_file(filename: &Path) -> Result<bool> {
    if let Some(dir) = filename.parent() {
        let created = ensure_dir(dir)?;
        if !filename.exists() {
            File::create(filename).map_err(|ex| Error::CannotCreateFile(ex.to_string()))?;
            Ok(true)
        } else {
            Ok(created)
        }
    } else {
        Err(Error::ImpossiblePath(
            filename.to_string_lossy().to_string(),
        ))
    }
}

pub fn delete_file(filename: &Path) -> Result<()> {
    if filename.exists() {
        fs::remove_file(filename).map_err(|ex| Error::CannotRemoveFile(ex.to_string()))?;
    }

    Ok(())
}

// endregion:     --- Files
