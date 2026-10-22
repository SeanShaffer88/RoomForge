//! Project file storage: atomic saves (write a temp file, then replace)
//! so a crash or power cut mid-save can never leave a torn plan behind,
//! plus the autosave location.

use crate::model::Project;
use std::path::{Path, PathBuf};

pub fn save_atomic(path: &Path, project: &Project) -> Result<(), String> {
    let text = project
        .to_json()
        .map_err(|e| format!("Could not prepare the plan to save: {e}"))?;
    let tmp = path.with_extension("roomforge.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    // std::fs::rename replaces the destination on Windows and Unix.
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("Could not finish saving {}: {e}", path.display()))?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    Project::from_json(&text)
}

/// `%LOCALAPPDATA%\RoomForge\autosave.roomforge` on Windows; a sensible
/// per-user fallback elsewhere. Used to recover work after a crash.
pub fn autosave_path() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.is_empty() {
            return PathBuf::from(local)
                .join("RoomForge")
                .join("autosave.roomforge");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".roomforge")
            .join("autosave.roomforge");
    }
    PathBuf::from("autosave.roomforge")
}

pub fn save_autosave(project: &Project) -> Result<(), String> {
    let path = autosave_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    save_atomic(&path, project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_save_then_load_roundtrips() {
        let dir = std::env::temp_dir().join(format!("roomforge-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plan.roomforge");
        let p = Project::default();
        save_atomic(&path, &p).unwrap();
        assert_eq!(load(&path).unwrap(), p);
        // The temp file must not be left behind.
        assert!(!dir.join("plan.roomforge.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loading_a_missing_file_is_a_clear_error() {
        let err = load(Path::new("/definitely/not/here.roomforge")).unwrap_err();
        assert!(err.contains("Could not read"));
    }
}
