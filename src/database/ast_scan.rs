use line_index::LineIndex;
use rust_analyzer_syntax::ast::{Enum, HasName, Module, Struct};
use rust_analyzer_syntax::{AstNode, SyntaxKind, SyntaxNode};
use snafu::{OptionExt, Whatever};

use crate::database::{Database, FileUrl, ItemPath, ModuleInclude, ModulePath, TypeDefData};

pub fn scan_ast(db: &mut Database, file: &FileUrl, index: &LineIndex, ast: SyntaxNode) {
    db.files
        .get_mut(file)
        .expect("failed to access file in AST scan")
        .modules
        .clear();

    match ast.kind() {
        SyntaxKind::MODULE => {
            let module = Module::cast(ast.clone()).expect("failed to cast module");
            _ = collect_module(db, file, index, module);
        }
        SyntaxKind::STRUCT => {
            let struc = Struct::cast(ast.clone()).expect("failed to cast struct");
            _ = collect_struct_def(db, file, index, struc);
        }
        SyntaxKind::ENUM => {
            let enu = Enum::cast(ast.clone()).expect("failed to cast enum");
            _ = collect_enum_def(db, file, index, enu);
        }
        _ => (),
    }

    for child in ast.children() {
        scan_ast(db, file, index, child);
    }
}

fn collect_module(
    db: &mut Database,
    file: &FileUrl,
    index: &LineIndex,
    module: Module,
) -> Result<(), Whatever> {
    let name = module
        .name()
        .whatever_context("module had no name")?
        .text_non_mutable()
        .to_string();
    let range = crate::utils::range(module.syntax().text_range(), index);
    db.files
        .get_mut(file)
        .expect("failed to access file while scanning")
        .modules
        .push(ModuleInclude { name, range });
    Ok(())
}

fn collect_struct_def(
    db: &mut Database,
    file: &FileUrl,
    index: &LineIndex,
    typedef: Struct,
) -> Result<(), Whatever> {
    let name = typedef
        .name()
        .whatever_context("struct definition had no name")?
        .text_non_mutable()
        .to_string();
    let range = crate::utils::range(typedef.syntax().text_range(), index);

    let item_path = ItemPath {
        module: ModulePath {
            crate_: "crate".to_string(),
            segments: vec![],
        },
        name: name.clone(),
    };
    let item_data = TypeDefData {
        file_path: file.clone(),
        range,
        name,
    };

    if let Some(old) = db.type_defs.insert(item_path, item_data) {
        db.log_warning(&format!(
            "discarding type def `{}`; conflicting type name encountered",
            old.name
        ));
    }

    Ok(())
}

fn collect_enum_def(
    db: &mut Database,
    file: &FileUrl,
    index: &LineIndex,
    typedef: Enum,
) -> Result<(), Whatever> {
    let name = typedef
        .name()
        .whatever_context("enum definition had no name")?
        .text_non_mutable()
        .to_string();
    let range = crate::utils::range(typedef.syntax().text_range(), index);

    let item_path = ItemPath {
        module: ModulePath {
            crate_: "crate".to_string(),
            segments: vec![],
        },
        name: name.clone(),
    };
    let item_data = TypeDefData {
        file_path: file.clone(),
        range,
        name,
    };

    if let Some(old) = db.type_defs.insert(item_path, item_data) {
        db.log_warning(&format!(
            "discarding type def `{}`; conflicting type name encountered",
            old.name
        ));
    }

    Ok(())
}
