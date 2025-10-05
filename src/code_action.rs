use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Position, Range, TextEdit,
    WorkspaceEdit,
};
use snafu::{OptionExt, ResultExt, Whatever};

use crate::database::{Database, FileUrl};

/// Add `[pub] mod {name};` to parent file
pub fn add_mod_to_parent(
    db: &mut Database,
    params: &CodeActionParams,
    actions: &mut Vec<CodeActionOrCommand>,
) -> Result<(), Whatever> {
    let file = FileUrl::from_url(params.text_document.uri.clone())?;

    let file_name = file.path().file_stem().expect("`FileUrl` had no filename");
    let file_name_str = file_name
        .to_str()
        .whatever_context("filename was not UTF8")?;
    db.load_file(&file)
        .with_whatever_context(|_| format!("failed to load file `{}`", file.url()))?;
    if let Some(parent_url) = &db
        .files
        .get(&file)
        .expect("successfully loaded file but it was not present in database")
        .parent
    {
        let parent = db
            .files
            .get(&parent_url)
            .expect("found parent URL but the parent file was not present in database");

        let last_include_range = parent
            .modules
            .last()
            .map(|include| Range::new(include.range.end, include.range.end))
            .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 0)));

        let parent_has_this_module = parent
            .modules
            .iter()
            .any(|module| &module.name[..] == file_name);
        if !parent_has_this_module {
            actions.push(insert_mod_private(
                file_name,
                file_name_str,
                last_include_range,
                parent_url,
            ));
            actions.push(insert_mod_public(
                file_name,
                file_name_str,
                last_include_range,
                parent_url,
            ));
        }
    }

    Ok(())
}

fn insert_mod_private(
    file_name: &std::ffi::OsStr,
    file_name_str: &str,
    last_include_range: Range,
    parent_url: &FileUrl,
) -> CodeActionOrCommand {
    let title = format!("Insert `mod {};`", file_name.to_string_lossy());
    let new_text = format!("\nmod {file_name_str};");
    insert_mod(title, new_text, last_include_range, parent_url)
}

fn insert_mod_public(
    file_name: &std::ffi::OsStr,
    file_name_str: &str,
    last_include_range: Range,
    parent_url: &FileUrl,
) -> CodeActionOrCommand {
    let title = format!("Insert `pub mod {};`", file_name.to_string_lossy());
    let new_text = format!("\npub mod {file_name_str};");
    insert_mod(title, new_text, last_include_range, parent_url)
}

fn insert_mod(
    title: String,
    new_text: String,
    last_include_range: Range,
    parent_url: &FileUrl,
) -> CodeActionOrCommand {
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from_iter([(
                parent_url.url().clone(),
                vec![TextEdit {
                    range: last_include_range,
                    new_text,
                }],
            )])),
            ..Default::default()
        }),
        ..Default::default()
    })
}
