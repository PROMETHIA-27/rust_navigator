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

use crate::database::Database;

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

    let (rid, params) = connection.initialize_start().unwrap();
    let mut initialize_params =
        serde_json::from_value::<InitializeParams>(params).unwrap_or_default();
    connection
        .initialize_finish(
            rid,
            serde_json::json!({
                "capabilities": server_capabilities,
            }),
        )
        .unwrap();

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
        let path = root.uri.to_file_path().unwrap();
        if let Err(err) = database::file::find_rust_files(&mut db, &path) {
            db.log_error(&format!("Rust-Navigator ERROR: {err}"));
        }
    }

    db.workspace_folders = workspace_folders;

    let mut shutdown = false;
    loop {
        let message = db.connection.receiver.recv().unwrap();
        match message {
            Message::Request(request) => {
                shutdown = db.connection.handle_shutdown(&request).unwrap_or_default();

                match &request.method[..] {
                    "textDocument/codeAction" => {
                        request::text_document::code_action(&mut db, request);
                    }
                    _ => (),
                }
            }
            Message::Response(_response) => todo!(),
            Message::Notification(notification) => match &notification.method[..] {
                "textDocument/didOpen" => {
                    notification::text_document::did_open(&mut db, notification)
                }
                "textDocument/didChange" => {
                    notification::text_document::did_change(&mut db, notification)
                }
                "textDocument/didClose" => {
                    notification::text_document::did_close(&mut db, notification)
                }
                _ => (),
            },
        };

        if shutdown {
            break;
        }
    }

    io_threads.join().unwrap();
}
