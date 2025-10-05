use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::Path;

use line_index::LineIndex;
use lsp_server::{Connection, Message, Notification};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams};
use rust_analyzer_syntax::{Parse, SourceFile};
use snafu::{ResultExt, Whatever};

use crate::database::{Database, FileUrl};
use crate::utils::OrLog;

pub fn get_file_diagnostics(ast: &Parse<SourceFile>, index: &LineIndex) -> Vec<Diagnostic> {
    let errors = ast
        .errors()
        .iter()
        .map(|error| {
            let range = ast
                .syntax_node()
                .covering_element(error.range())
                .text_range();
            Diagnostic {
                severity: Some(DiagnosticSeverity::ERROR),
                range: crate::utils::range(range, index),
                message: format!("Syntax Error: {error}"),
                ..Default::default()
            }
        })
        .collect();

    errors
}

pub fn post_diagnostics(
    connection: &Connection,
    file: &FileUrl,
    diagnostics: Vec<Diagnostic>,
    version: i32,
) {
    connection
        .sender
        .send(Message::Notification(Notification {
            method: PublishDiagnostics::METHOD.to_string(),
            params: serde_json::to_value(PublishDiagnosticsParams {
                uri: file.url().clone(),
                diagnostics,
                version: Some(version),
            })
            .expect("failed to convert PublishDiagnosticParams to json value"),
        }))
        .expect("failed to push diagnostics");
}

pub fn find_rust_files(db: &mut Database, root: &Path) -> Result<(), Whatever> {
    let dir = std::fs::read_dir(root)
        .with_whatever_context(|_| format!("failed to load path {root:?}"))?;

    // Look for markers like `CACHEDIR.TAG` first
    let file_buffer = dir.filter_map(|file| file.ok()).collect::<Vec<DirEntry>>();
    let mut should_skip = false;
    for entry in file_buffer.iter() {
        should_skip |= should_skip_dir(entry);
    }

    if should_skip {
        return Ok(());
    }

    for file in file_buffer {
        let file_path = file.path();
        let Some(ty) = file
            .file_type()
            .with_whatever_context::<_, _, Whatever>(|_| {
                format!("could not get file type for {file_path:?}")
            })
            .or_log(db)
        else {
            continue;
        };

        if ty.is_dir() {
            if file_path.file_name() == Some(OsStr::new(".git")) {
                continue;
            }

            _ = find_rust_files(db, &file_path).or_log(db);
        } else if ty.is_file() {
            if file_path.extension() != Some(OsStr::new("rs")) {
                continue;
            }

            let Some(file) = FileUrl::from_path(&file_path).or_log(db) else {
                continue;
            };
            _ = db
                .load_file(&file)
                .with_whatever_context::<_, _, Whatever>(|_| {
                    format!("failed to load file {}", file.url())
                })
                .or_log(db);
        } else if ty.is_symlink() {
            db.log_error("symlinks aren't supported right now, try again later");
        }
    }

    Ok(())
}

fn should_skip_dir(entry: &DirEntry) -> bool {
    if let Ok(ty) = entry.file_type() {
        if ty.is_file() {
            return entry.file_name() == OsStr::new("CACHEDIR.TAG");
        }
    }
    return false;
}
