//! Paths del workspace de caso.
//!
//! Layout:
//! ```text
//! <workspace_root>/<case_id>/
//! ├── case.json
//! ├── audit.log
//! ├── .lock
//! ├── evidence/<evidence_id>/{pristine,working}/
//! ├── exports/
//! └── tool_version.txt
//! ```

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CasePaths {
    pub case_dir: PathBuf,
    pub case_json: PathBuf,
    pub audit_log: PathBuf,
    pub lock_file: PathBuf,
    pub evidence_root: PathBuf,
    pub exports_dir: PathBuf,
    pub tool_version_file: PathBuf,
}

impl CasePaths {
    pub fn new(workspace_root: &Path, case_id: &str) -> Self {
        let case_dir = workspace_root.join(case_id);
        Self {
            case_json: case_dir.join("case.json"),
            audit_log: case_dir.join("audit.log"),
            lock_file: case_dir.join(".lock"),
            evidence_root: case_dir.join("evidence"),
            exports_dir: case_dir.join("exports"),
            tool_version_file: case_dir.join("tool_version.txt"),
            case_dir,
        }
    }

    pub fn evidence_dir(&self, evidence_id: &str) -> PathBuf {
        self.evidence_root.join(evidence_id)
    }
}

/// Workspace root por defecto en el sistema de archivos del usuario.
///
/// Linux: `~/.local/share/whatsforensics/workspaces`
/// Windows: `%APPDATA%/whatsforensics/workspaces`
/// macOS: `~/Library/Application Support/whatsforensics/workspaces`
pub fn default_workspace_root() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("whatsforensics").join("workspaces"))
}
