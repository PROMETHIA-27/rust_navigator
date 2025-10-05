pub mod code_action;
pub mod database;
pub mod notification;
pub mod request;
pub mod utils;

use std::collections::HashMap;

use lsp_server::{Connection, Message};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    InitializeParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};
use snafu::{OptionExt, ResultExt, Whatever};

use crate::database::Database;
use crate::utils::OrLog;

fn main() {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: None,
            inter_file_dependencies: true,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        })),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    };

    let (rid, params) = connection
        .initialize_start()
        .expect("initialization failed");
    let mut initialize_params =
        serde_json::from_value::<InitializeParams>(params).unwrap_or_default();
    connection
        .initialize_finish(
            rid,
            serde_json::json!({
                "capabilities": server_capabilities,
            }),
        )
        .expect("initialization finish failed");

    let workspace_folders = initialize_params
        .workspace_folders
        .take()
        .unwrap_or_default();

    let mut db = Database {
        connection,
        initialize_params,
        workspace_folders,
        files: HashMap::default(),
        modules: HashMap::default(),
        type_defs: HashMap::default(),
    };

    db.log_info("Rust-Navigator Loaded");

    let workspace_folders = std::mem::take(&mut db.workspace_folders);

    for root in &workspace_folders {
        let Some(path) = root
            .uri
            .to_file_path()
            .ok()
            .with_whatever_context::<_, _, Whatever>(|| {
                format!("failed to convert root URI {} to file path", root.uri)
            })
            .or_log(&db)
        else {
            continue;
        };
        if let Err(err) = database::file::find_rust_files(&mut db, &path) {
            db.log_error(&format!("Rust-Navigator ERROR: {err}"));
        }
    }

    db.workspace_folders = workspace_folders;

    let mut shutdown = false;
    loop {
        let Some(message) = db
            .connection
            .receiver
            .recv()
            .whatever_context::<_, Whatever>("failed to receive message")
            .or_log(&db)
        else {
            continue;
        };
        match message {
            Message::Request(request) => {
                shutdown = db.connection.handle_shutdown(&request).unwrap_or_default();

                match &request.method[..] {
                    "textDocument/codeAction" => {
                        _ = request::text_document::code_action(&mut db, request).or_log(&db);
                    }
                    _ => (),
                }
            }
            Message::Response(_response) => todo!(),
            Message::Notification(notification) => match &notification.method[..] {
                "textDocument/didOpen" => {
                    _ = notification::text_document::did_open(&mut db, notification).or_log(&db);
                }
                "textDocument/didChange" => {
                    _ = notification::text_document::did_change(&mut db, notification).or_log(&db);
                }
                "textDocument/didClose" => {
                    _ = notification::text_document::did_close(&mut db, notification).or_log(&db);
                }
                _ => (),
            },
        };

        if shutdown {
            break;
        }
    }

    io_threads.join().expect("failed to join IO threads");
}
