use lsp_server::Notification;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};
use snafu::{OptionExt, ResultExt, Whatever};

use crate::database::{Database, FileUrl};

pub fn did_open(db: &mut Database, notification: Notification) -> Result<(), Whatever> {
    let params = serde_json::from_value::<DidOpenTextDocumentParams>(notification.params)
        .whatever_context("received invalid textDocument/didOpen params")?;

    let path = FileUrl::from_url(params.text_document.uri)
        .whatever_context("didOpen file URI failed to convert to a path")?;
    db.files.entry(path.clone()).or_default();
    db.update_file(
        &path,
        params.text_document.version,
        &params.text_document.text,
    );

    Ok(())
}

pub fn did_change(db: &mut Database, notification: Notification) -> Result<(), Whatever> {
    let params = serde_json::from_value::<DidChangeTextDocumentParams>(notification.params)
        .whatever_context("received invalid textDocument/didChange params")?;

    let src = &params.content_changes[0].text;
    let file_url = FileUrl::from_url(params.text_document.uri)
        .whatever_context("didChange file URI failed to convert to a path")?;
    db.update_file(&file_url, params.text_document.version, src);

    Ok(())
}

pub fn did_close(db: &mut Database, notification: Notification) -> Result<(), Whatever> {
    let params = serde_json::from_value::<DidCloseTextDocumentParams>(notification.params)
        .whatever_context("received invalid textDocument/didClose params")?;

    let file_url = FileUrl::from_url(params.text_document.uri)
        .whatever_context("didClose file URI failed to convert to a path")?;
    db.files
        .get_mut(&file_url)
        .with_whatever_context(|| {
            format!("file {} could not be accessed to close", file_url.url())
        })?
        .is_open = false;

    Ok(())
}
