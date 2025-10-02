use lsp_server::{Message, Request, Response};
use lsp_types::CodeActionParams;

use crate::code_action;
use crate::database::Database;

pub fn code_action(db: &mut Database, request: Request) {
    let params = serde_json::from_value::<CodeActionParams>(request.params).unwrap();

    let mut actions = vec![];
    code_action::add_mod_to_parent(db, &params, &mut actions);

    let value = serde_json::to_value(actions).unwrap();

    db.connection
        .sender
        .send(Message::Response(Response {
            id: request.id,
            result: Some(value),
            error: None,
        }))
        .unwrap();
}
