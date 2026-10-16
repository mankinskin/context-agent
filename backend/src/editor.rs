use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use crate::AppState;

const EDITOR_FILE: &str = "notes/demo.txt";
const MAX_BYTES: usize = 65_536;
#[path = "editor_git.rs"]
mod git;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileQuery {
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveRequest {
    path: String,
    content: String,
}

#[derive(Serialize)]
pub(crate) struct FileResponse {
    path: &'static str,
    content: String,
}

#[derive(Serialize)]
pub(crate) struct DiffResponse {
    diff: String,
}

type EditorResult<T> = Result<Json<T>, (StatusCode, String)>;

pub(crate) fn error_detail(code: &str) -> Option<&'static str> {
    match code {
        "editor_path_rejected" => Some("Open only the existing UTF-8 notes/demo.txt (at most 65536 bytes) in the configured owned repository; links and alternate roots are forbidden."),
        "editor_root_disabled" => Some("The operator must configure CONTEXT_AGENT_EDITOR_ROOT with an owned temporary Git repository."),
        code if code.starts_with("editor_git_") => Some("Ask the operator to check Git availability, the owned repository and server logs; Git commands are bounded to 3 seconds and limited output."),
        "editor_write_failed" | "editor_cleanup_failed" => Some("The save failed. Keep the draft and ask the operator to inspect file permissions and the correlated server log before retrying."),
        _ => None,
    }
}

fn rejected<T>() -> Result<T, (StatusCode, String)> {
    Err((StatusCode::BAD_REQUEST, "editor_path_rejected".into()))
}

fn editor_root(state: &AppState) -> Result<&Path, (StatusCode, String)> {
    state.editor_root.as_deref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "editor_root_disabled".into(),
    ))
}

fn has_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_attributes() & 0x400 != 0)
            .unwrap_or(true)
    }
    #[cfg(not(windows))]
    {
        fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
    }
}

fn checked_file(root: &Path, requested: &str) -> Result<PathBuf, (StatusCode, String)> {
    if requested != EDITOR_FILE
        || requested.contains('\\')
        || requested.contains('%')
        || requested.contains(':')
        || requested.starts_with('/')
        || requested.starts_with("//")
    {
        return rejected();
    }
    let relative = Path::new(requested);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return rejected();
    }
    if has_reparse_point(root) {
        return rejected();
    }
    let root = fs::canonicalize(root).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "editor_root_disabled".into(),
        )
    })?;
    let parent = root.join("notes");
    let file = parent.join("demo.txt");
    if has_reparse_point(&parent)
        || has_reparse_point(&file)
        || !fs::symlink_metadata(&file).is_ok_and(|metadata| metadata.is_file())
    {
        return rejected();
    }
    let canonical_parent = fs::canonicalize(&parent)
        .map_err(|_| (StatusCode::BAD_REQUEST, "editor_path_rejected".into()))?;
    let canonical_file = fs::canonicalize(&file)
        .map_err(|_| (StatusCode::BAD_REQUEST, "editor_path_rejected".into()))?;
    if canonical_parent.parent() != Some(root.as_path())
        || canonical_file.parent() != Some(canonical_parent.as_path())
        || !canonical_file.starts_with(&root)
    {
        return rejected();
    }
    Ok(canonical_file)
}

fn checked_root(root: &Path) -> Result<PathBuf, (StatusCode, String)> {
    if has_reparse_point(root) {
        return rejected();
    }
    let root = fs::canonicalize(root).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "editor_root_disabled".into(),
        )
    })?;
    if !fs::symlink_metadata(root.join(".git")).is_ok_and(|metadata| metadata.is_dir())
        || has_reparse_point(&root.join(".git"))
    {
        return rejected();
    }
    let output = git::run(&root, &["rev-parse", "--show-toplevel"], 8192)?;
    let git_root = String::from_utf8(output)
        .ok()
        .and_then(|value| fs::canonicalize(value.trim()).ok())
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "editor_git_unavailable".into(),
        ))?;
    if git_root != root || has_reparse_point(&root.join(".git")) {
        return rejected();
    }
    Ok(root)
}

pub(crate) async fn open(
    State(state): State<AppState>,
    Query(query): Query<FileQuery>,
) -> EditorResult<FileResponse> {
    let _transaction = state.transaction.lock().await;
    let root = checked_root(editor_root(&state)?)?;
    let file = checked_file(&root, &query.path)?;
    let mut bytes = Vec::new();
    fs::File::open(file)
        .and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| {
            tracing::warn!(%error, "editor file read failed");
            (StatusCode::BAD_REQUEST, "editor_path_rejected".into())
        })?;
    if bytes.len() > MAX_BYTES {
        return rejected();
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, "editor_path_rejected".into()))?;
    Ok(Json(FileResponse {
        path: EDITOR_FILE,
        content,
    }))
}

pub(crate) async fn save(
    State(state): State<AppState>,
    Json(request): Json<SaveRequest>,
) -> EditorResult<FileResponse> {
    let _transaction = state.transaction.lock().await;
    if request.content.len() > MAX_BYTES {
        return rejected();
    }
    let root = checked_root(editor_root(&state)?)?;
    let file = checked_file(&root, &request.path)?;
    let parent = file
        .parent()
        .ok_or((StatusCode::BAD_REQUEST, "editor_path_rejected".into()))?;
    let temporary = parent.join(format!(".editor-{}.tmp", uuid::Uuid::new_v4()));
    let write = (|| -> std::io::Result<()> {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        output.write_all(request.content.as_bytes())?;
        output.sync_all()?;
        drop(output);
        checked_file(&root, &request.path)
            .map_err(|_| std::io::Error::other("editor path changed"))?;
        fs::rename(&temporary, &file)
    })();
    if let Err(error) = write {
        tracing::error!(%error, "editor atomic replacement failed");
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|cleanup| {
                tracing::error!(%cleanup, "editor temporary cleanup failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "editor_cleanup_failed".to_string(),
                )
            })?;
        }
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "editor_write_failed".into(),
        ));
    }
    Ok(Json(FileResponse {
        path: EDITOR_FILE,
        content: request.content,
    }))
}

pub(crate) async fn diff(State(state): State<AppState>) -> EditorResult<DiffResponse> {
    let _transaction = state.transaction.lock().await;
    let root = checked_root(editor_root(&state)?)?;
    checked_file(&root, EDITOR_FILE)?;
    let output = git::run(
        &root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--",
            EDITOR_FILE,
        ],
        262_144,
    )?;
    let diff = String::from_utf8(output).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "editor_git_unavailable".into(),
        )
    })?;
    Ok(Json(DiffResponse { diff }))
}
