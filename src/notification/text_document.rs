use lsp_server::Notification;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use crate::database::{Database, FileUrl};

pub fn did_open(db: &mut Database, notification: Notification) {
    let params = serde_json::from_value::<DidOpenTextDocumentParams>(notification.params).unwrap();

    let path = FileUrl::from_url(params.text_document.uri);
    db.files.entry(path.clone()).or_default();
    db.update_file(
        &path,
        params.text_document.version,
        &params.text_document.text,
    );
}

pub fn did_change(db: &mut Database, notification: Notification) {
    let params =
        serde_json::from_value::<DidChangeTextDocumentParams>(notification.params).unwrap();

    let src = &params.content_changes[0].text;
    db.update_file(
        &FileUrl::from_url(params.text_document.uri),
        params.text_document.version,
        src,
    );
}

pub fn did_close(db: &mut Database, notification: Notification) {
    let params = serde_json::from_value::<DidCloseTextDocumentParams>(notification.params).unwrap();

    db.files
        .get_mut(&FileUrl::from_url(params.text_document.uri))
        .unwrap()
        .is_open = false;
}
