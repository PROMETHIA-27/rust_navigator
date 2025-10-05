use crate::database::{Database, FileUrl};

pub fn scan_file_modules(db: &mut Database, file: &FileUrl) {
    db.files
        .get_mut(file)
        .expect("failed to access file during update")
        .parent = get_parent_uri(db, file);
}

/// Check if an adjacent file named `mod.rs`, `lib.rs`, or `main.rs`,
/// or a parent file named after this directory exists,
/// and if so return its uri. The file will be loaded in the database if returned.
///
/// If this file is a `mod.rs` file, then it will check from a directory up.
fn get_parent_uri(db: &mut Database, file: &FileUrl) -> Option<FileUrl> {
    let path = file.path();
    let file_name = path.file_name().expect("file had no filename");

    if file_name == "lib.rs" || file_name == "main.rs" {
        return None;
    }

    let path = if file_name != "mod.rs" {
        &path
    } else {
        path.parent()?
    };

    let mod_path = path.with_file_name("mod.rs");
    if let Ok(mod_url) = FileUrl::from_path(&mod_path)
        && let Some(_) = db.get_file(&mod_url)
    {
        return Some(mod_url);
    }

    let lib_path = path.with_file_name("lib.rs");

    if let Ok(lib_url) = FileUrl::from_path(&lib_path)
        && let Some(_) = db.get_file(&lib_url)
    {
        return Some(lib_url);
    }

    let main_path = path.with_file_name("main.rs");

    if let Ok(main_url) = FileUrl::from_path(&main_path)
        && let Some(_) = db.get_file(&main_url)
    {
        return Some(main_url);
    }

    let parent_dir = path.parent()?;
    let parent_file = parent_dir.with_extension("rs");

    if let Ok(parent_url) = FileUrl::from_path(&parent_file)
        && let Some(_) = db.get_file(&parent_url)
    {
        return Some(parent_url);
    }

    None
}
