pub mod ast_scan;
pub mod file;
pub mod module;

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::panic::Location;
use std::path::{Path, PathBuf};

use line_index::LineIndex;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{InitializeParams, MessageType, Range, Url, WorkspaceFolder};
use rust_analyzer_syntax::{Edition, Parse, SourceFile};
use serde_json::json;
use snafu::{OptionExt, ResultExt, Whatever};

use crate::database::ast_scan::scan_ast;
use crate::database::file::{get_file_diagnostics, post_diagnostics};
use crate::database::module::scan_file_modules;

/// A canonicalized path with its URL generated and saved ahead of time, since almost every usage
/// of a path will at some point require producing a URL from it (for this LSP).
///
/// On windows, the canonicalized path will be a literal path. More on that [here](https://www.fileside.app/blog/2023-03-17_windows-file-paths/)
///
/// Some properties of canonicalized paths:
/// - No `.` or `..`, these have been resolved
/// - On windows, no `/`, as these have been replaced with `\`
/// - No repeated consecutive slashes
/// - No symbolic links, these have been resolved
/// - No trailing spaces (at least on windows?)
///
/// Some implications for operations on canonicalized paths:
/// - *Removing* path segments produces a canonicalized path as long as a trailing slash is left,
///   otherwise the path could point to a file named the same as a directory segment and end up
///   hitting a symbolic link
/// - *Adding* path segments does not; as an added path segment could lead to a symbolic link
///   - Likewise with changing path segments
#[derive(Clone, Debug, Eq)]
pub struct FileUrl(PathBuf, Url);

impl FileUrl {
    pub fn from_path(path: &Path) -> Result<FileUrl, Whatever> {
        let path = path
            .canonicalize()
            .with_whatever_context(|_| format!("failed to canonicalize path {path:?}"))?;
        let url = Url::from_file_path(&path).expect("canonicalized file was not a valid file path");
        Ok(FileUrl(path, url))
    }

    /// Can fail if the URL was not a `file` url, or failed to canonicalize
    pub fn from_url(url: Url) -> Result<FileUrl, Whatever> {
        let path = url
            .to_file_path()
            .ok()
            .with_whatever_context(|| format!("failed to convert URL {url} to a filepath"))?
            .canonicalize()
            .with_whatever_context(|_| format!("failed to canonicalize url {url}"))?;
        Ok(FileUrl(path, url))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn url(&self) -> &Url {
        &self.1
    }
}

impl Hash for FileUrl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl PartialEq for FileUrl {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub struct Database {
    pub connection: Connection,
    pub initialize_params: InitializeParams,
    pub workspace_folders: Vec<WorkspaceFolder>,
    pub files: HashMap<FileUrl, FileData>,
    pub modules: HashMap<ModulePath, ModuleData>,
    pub type_defs: HashMap<ItemPath, TypeDefData>,
}

impl Database {
    #[track_caller]
    pub fn log_info(&self, message: &str) {
        self.log_internal(MessageType::INFO, message, Location::caller());
    }

    #[track_caller]
    pub fn log_warning(&self, message: &str) {
        self.log_internal(MessageType::WARNING, message, Location::caller());
    }

    #[track_caller]
    pub fn log_error(&self, message: &str) {
        self.log_internal(MessageType::ERROR, message, Location::caller());
    }

    fn log_internal(&self, message_type: MessageType, message: &str, location: &Location) {
        self.connection
            .sender
            .send(Message::Notification(Notification {
                method: "window/logMessage".to_string(),
                params: json!({
                    "type": message_type,
                    "message": format!("{}: {}", location, message),
                }),
            }))
            .expect("failed to send log");
    }

    /// Attempt to check a file from the database, and if it's missing, load it from the filesystem
    pub fn get_file(&mut self, file: &FileUrl) -> Option<&FileData> {
        if !self.files.contains_key(file) {
            self.load_file(file).ok()?;
        }

        self.files.get(file)
    }

    /// If the file is missing from the database, load it from the filesystem and update it
    pub fn load_file(&mut self, file: &FileUrl) -> std::io::Result<()> {
        // Don't update unless the file is missing; otherwise this will cause a lot of unnecessary
        // recomputes
        if self.files.get(file).is_none() {
            let file_src = std::fs::read_to_string(file.path())?;
            let data = FileData::default();
            let version = data.version;
            self.files.insert(file.clone(), data);
            self.update_file(&file, version, &file_src);
        }

        Ok(())
    }

    /// Assume the FileData exists and reprocess it
    pub fn update_file(&mut self, file: &FileUrl, version: i32, src: &str) {
        let line_index = LineIndex::new(src);

        let ast = rust_analyzer_syntax::SourceFile::parse(src, Edition::Edition2024);

        let diagnostics = get_file_diagnostics(&ast, &line_index);
        post_diagnostics(&self.connection, &file, diagnostics, version);

        let file_data = self.files.get_mut(file).expect("file data missing");

        for type_def in &file_data.types {
            self.type_defs.remove(type_def);
        }

        scan_file_modules(self, file);
        scan_ast(self, file, &line_index, ast.syntax_node());

        let file_data = self.files.get_mut(file).expect("file data missing");
        file_data.version = version;
        file_data.index = line_index;
        file_data.ast = ast;
    }
}

pub struct FileData {
    pub version: i32,
    pub index: LineIndex,
    pub is_open: bool,
    pub modules: Vec<ModuleInclude>,
    pub parent: Option<FileUrl>,
    pub ast: Parse<SourceFile>,
    /// Used to clear old type defs when a file is changed/removed
    pub types: Vec<ItemPath>,
}

impl Default for FileData {
    fn default() -> Self {
        Self {
            version: 0,
            index: LineIndex::new(""),
            is_open: false,
            modules: vec![],
            parent: None,
            ast: SourceFile::parse("", Edition::Edition2015),
            types: vec![],
        }
    }
}

#[derive(Debug)]
pub struct ModuleInclude {
    pub name: String,
    pub range: Range,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ModulePath {
    pub crate_: String,
    pub segments: Vec<String>,
}

pub struct ModuleData {
    pub name: String,
    pub children: Vec<String>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ItemPath {
    pub module: ModulePath,
    pub name: String,
}

#[derive(Debug)]
pub struct TypeDefData {
    pub file_path: FileUrl,
    pub range: Range,
    pub name: String,
}
