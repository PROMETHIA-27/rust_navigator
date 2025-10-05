use lsp_server::{Message, Request, Response};
use lsp_types::CodeActionParams;
use snafu::{ResultExt, Whatever};

use crate::code_action;
use crate::database::Database;

pub fn code_action(db: &mut Database, request: Request) -> Result<(), Whatever> {
    let params = serde_json::from_value::<CodeActionParams>(request.params)
        .whatever_context("received invalid textDocument/codeAction params")?;

    let mut actions = vec![];
    code_action::add_mod_to_parent(db, &params, &mut actions)
        .whatever_context("failed to add `mod` import to parent file")?;

    let value = serde_json::to_value(actions).expect("failed to turn CodeAction vec to json value");

    db.connection
        .sender
        .send(Message::Response(Response {
            id: request.id,
            result: Some(value),
            error: None,
        }))
        .whatever_context("failed to send codeAction response")?;

    Ok(())
}
