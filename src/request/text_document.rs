use line_index::{TextRange, TextSize};
use lsp_server::{ErrorCode, Message, Request, Response, ResponseError};
use lsp_types::{CodeActionParams, GotoDefinitionParams};
use snafu::{OptionExt, ResultExt, Whatever};

use crate::code_action;
use crate::database::{Database, FileUrl, ItemPath, ModulePath};
use crate::utils::line_col;

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

pub fn definition(db: &mut Database, request: Request) -> Result<(), Whatever> {
    let params = serde_json::from_value::<GotoDefinitionParams>(request.params)
        .whatever_context("received invalid textDocument/definition params")?;

    let text_doc = db
        .get_file(&FileUrl::from_url(
            params.text_document_position_params.text_document.uri,
        )?)
        .whatever_context("failed to get definition request file")?;
    let offset = text_doc
        .index
        .offset(line_col(params.text_document_position_params.position))
        .whatever_context("failed to get position in definition request")?;
    let range = TextRange::at(offset, TextSize::default());
    let target_node = text_doc.ast.syntax_node().covering_element(range);

    let name = target_node
        .as_token()
        .expect("result of covering element was not a token")
        .text();

    let path = ItemPath {
        module: ModulePath {
            crate_: "crate".to_string(),
            segments: vec![],
        },
        name: name.to_string(),
    };

    let result = db
        .type_defs
        .get(&path)
        .map(|data| lsp_types::Location::new(data.file_path.url().clone(), data.range))
        .map(|loc| serde_json::to_value(loc).expect("failed to turn location into json value"))
        .or_else(|| {
            db.function_defs
                .get(&path)
                .map(|data| lsp_types::Location::new(data.file_path.url().clone(), data.range))
                .map(|loc| {
                    serde_json::to_value(loc).expect("failed to turn location into json value")
                })
        });

    let error = if result.is_none() {
        Some(ResponseError {
            code: ErrorCode::RequestFailed as i32,
            message: "No definition found".to_string(),
            data: None,
        })
    } else {
        None
    };

    db.connection
        .sender
        .send(Message::Response(Response {
            id: request.id,
            result,
            error,
        }))
        .whatever_context("failed to send definition response")?;

    Ok(())
}
