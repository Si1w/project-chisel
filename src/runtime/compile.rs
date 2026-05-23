use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::runtime::bootstrap::bootstrap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: String,
    pub path: Option<PathBuf>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileReport {
    diagnostics: Vec<Diagnostic>,
}

impl CompileReport {
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    }
}

#[must_use]
pub fn compile_project(root: &Path) -> CompileReport {
    match bootstrap(root) {
        Ok(_state) => CompileReport {
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Note,
                code: "compile-ok".into(),
                path: None,
                message: format!("project compiled: {}", root.display()),
            }],
        },
        Err(error) => CompileReport {
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Error,
                code: "compile-failed".into(),
                path: None,
                message: format!("{error:#}"),
            }],
        },
    }
}
