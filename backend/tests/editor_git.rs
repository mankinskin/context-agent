use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tower::ServiceExt;

struct Fixture {
    _owner: tempfile::TempDir,
    root: PathBuf,
    sentinel: PathBuf,
    app: Router,
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Editor Fixture",
            "-c",
            "user.email=editor@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> Fixture {
    let owner = tempfile::tempdir().unwrap();
    let root = owner.path().join("owned-git");
    fs::create_dir_all(root.join("notes")).unwrap();
    fs::write(root.join("notes/demo.txt"), "original fixture text\n").unwrap();
    let outside = owner.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("demo.txt");
    fs::write(&sentinel, "EXTERNAL_SENTINEL_UNCHANGED\n").unwrap();
    git(&root, &["init"]);
    git(&root, &["add", "notes/demo.txt"]);
    git(&root, &["commit", "-m", "owned editor fixture"]);
    let app = backend::router_with_editor("", owner.path().join("native"), root.clone());
    Fixture {
        _owner: owner,
        root,
        sentinel,
        app,
    }
}

async fn call(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn editor_reads_saves_and_reports_actual_git_diff_without_external_helpers() {
    let fixture = fixture();
    let (status, opened) = call(
        &fixture.app,
        "GET",
        "/api/editor/file?path=notes/demo.txt",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(opened["path"], "notes/demo.txt");
    assert_eq!(opened["content"], "original fixture text\n");
    fs::write(
        fixture.root.join(".gitattributes"),
        "notes/demo.txt diff=hostile\n",
    )
    .unwrap();
    git(&fixture.root, &["config", "diff.external", "git --version"]);
    git(
        &fixture.root,
        &["config", "diff.hostile.textconv", "git --version"],
    );
    let content = "original fixture text\nEDITOR_PROOF_29AD\n";
    let (status, saved) = call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":content}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        fs::read_to_string(fixture.root.join("notes/demo.txt")).unwrap(),
        content
    );
    let (status, diff) = call(&fixture.app, "GET", "/api/editor/diff", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{diff}");
    assert!(diff["diff"]
        .as_str()
        .unwrap()
        .contains("+EDITOR_PROOF_29AD"));
    assert!(diff["diff"].as_str().unwrap().contains("notes/demo.txt"));
    assert_eq!(
        fs::read_to_string(&fixture.sentinel).unwrap(),
        "EXTERNAL_SENTINEL_UNCHANGED\n"
    );
    assert_eq!(fs::read_dir(fixture.root.join("notes")).unwrap().count(), 1);
}

#[tokio::test]
async fn editor_rejects_traversal_absolute_encoded_git_foreign_and_missing_paths() {
    let fixture = fixture();
    for path in [
        "../outside/demo.txt",
        "/etc/passwd",
        "C:/Windows/win.ini",
        "%2e%2e/outside/demo.txt",
        "notes/%2e%2e/demo.txt",
        "notes\\demo.txt",
        ".git/config",
        "notes/missing.txt",
        "notes/demo.txt:stream",
        "//server/share/file",
    ] {
        let (status, error) = call(
            &fixture.app,
            "PUT",
            "/api/editor/file",
            json!({"path":path,"content":"FORBIDDEN"}),
        )
        .await;
        assert!(status.is_client_error(), "{path}: {status} {error}");
        assert!(error["correlation_id"].is_string(), "{error}");
    }
    for path in [
        "%2e%2e/outside/demo.txt",
        "%252e%252e/outside/demo.txt",
        "%2egit/config",
    ] {
        let (status, _) = call(
            &fixture.app,
            "GET",
            &format!("/api/editor/file?path={path}"),
            Value::Null,
        )
        .await;
        assert!(status.is_client_error(), "{path}: {status}");
    }
    let (status, _) = call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":"FORBIDDEN","workspace":"foreign"}),
    )
    .await;
    assert!(status.is_client_error());
    assert_eq!(
        fs::read_to_string(fixture.root.join("notes/demo.txt")).unwrap(),
        "original fixture text\n"
    );
    assert_eq!(
        fs::read_to_string(&fixture.sentinel).unwrap(),
        "EXTERNAL_SENTINEL_UNCHANGED\n"
    );
}

#[tokio::test]
async fn editor_requires_explicit_root_and_existing_bounded_utf8_file() {
    let fixture = fixture();
    let disabled = backend::router_with_store("", fixture._owner.path().join("disabled-native"));
    assert_eq!(
        call(
            &disabled,
            "GET",
            "/api/editor/file?path=notes/demo.txt",
            Value::Null
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let (status, _) = call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":"X".repeat(65_537)}),
    )
    .await;
    assert!(status.is_client_error());
    fs::write(fixture.root.join("notes/demo.txt"), [0xff, 0xfe]).unwrap();
    assert!(call(
        &fixture.app,
        "GET",
        "/api/editor/file?path=notes/demo.txt",
        Value::Null
    )
    .await
    .0
    .is_client_error());
    fs::remove_file(fixture.root.join("notes/demo.txt")).unwrap();
    assert!(call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":"NO_CREATE"})
    )
    .await
    .0
    .is_client_error());
    assert!(!fixture.root.join("notes/demo.txt").exists());
}

fn directory_link(target: &Path, link: &Path) {
    #[cfg(windows)]
    {
        let output = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "mandatory junction fixture unavailable: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[tokio::test]
async fn editor_rejects_parent_link_escape_and_preserves_external_sentinel() {
    let fixture = fixture();
    fs::rename(fixture.root.join("notes"), fixture.root.join("owned-notes")).unwrap();
    let link = fixture.root.join("notes");
    directory_link(fixture.sentinel.parent().unwrap(), &link);
    let read = call(
        &fixture.app,
        "GET",
        "/api/editor/file?path=notes/demo.txt",
        Value::Null,
    )
    .await;
    let save = call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":"FORBIDDEN"}),
    )
    .await;
    #[cfg(windows)]
    fs::remove_dir(&link).unwrap();
    #[cfg(unix)]
    fs::remove_file(&link).unwrap();
    assert!(read.0.is_client_error(), "{read:?}");
    assert!(save.0.is_client_error(), "{save:?}");
    assert!(call(&fixture.app, "GET", "/api/editor/diff", Value::Null)
        .await
        .0
        .is_client_error());
    assert_eq!(
        fs::read_to_string(&fixture.sentinel).unwrap(),
        "EXTERNAL_SENTINEL_UNCHANGED\n"
    );
}

#[tokio::test]
async fn editor_rejects_configured_root_link_and_oversized_file_read() {
    let fixture = fixture();
    fs::write(fixture.root.join("notes/demo.txt"), vec![b'X'; 65_537]).unwrap();
    let (status, error) = call(
        &fixture.app,
        "GET",
        "/api/editor/file?path=notes/demo.txt",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["code"], "editor_path_rejected");
    assert!(error["details"][0].as_str().unwrap().contains("65536"));
    let link = fixture._owner.path().join("linked-root");
    directory_link(&fixture.root, &link);
    let app = backend::router_with_editor(
        "",
        fixture._owner.path().join("linked-native"),
        link.clone(),
    );
    for uri in ["/api/editor/file?path=notes/demo.txt", "/api/editor/diff"] {
        assert_eq!(
            call(&app, "GET", uri, Value::Null).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    #[cfg(windows)]
    fs::remove_dir(&link).unwrap();
    #[cfg(unix)]
    fs::remove_file(&link).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn editor_rejects_file_symlink_escape() {
    let fixture = fixture();
    fs::remove_file(fixture.root.join("notes/demo.txt")).unwrap();
    std::os::unix::fs::symlink(&fixture.sentinel, fixture.root.join("notes/demo.txt")).unwrap();
    assert!(call(
        &fixture.app,
        "GET",
        "/api/editor/file?path=notes/demo.txt",
        Value::Null
    )
    .await
    .0
    .is_client_error());
    assert!(call(
        &fixture.app,
        "PUT",
        "/api/editor/file",
        json!({"path":"notes/demo.txt","content":"FORBIDDEN"})
    )
    .await
    .0
    .is_client_error());
    assert_eq!(
        fs::read_to_string(&fixture.sentinel).unwrap(),
        "EXTERNAL_SENTINEL_UNCHANGED\n"
    );
}

#[tokio::test]
async fn editor_atomic_replacement_never_exposes_partial_content() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let fixture = fixture();
    let path = fixture.root.join("notes/demo.txt");
    let a = "A".repeat(32_768);
    let b = "B".repeat(32_768);
    fs::write(&path, &a).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let reader_stop = stop.clone();
    let reader = std::thread::spawn(move || {
        while !reader_stop.load(Ordering::Relaxed) {
            let bytes = fs::read(&path).unwrap();
            assert!(
                bytes.len() == 32_768
                    && (bytes.iter().all(|byte| *byte == b'A')
                        || bytes.iter().all(|byte| *byte == b'B'))
            );
        }
    });
    for content in [&b, &a, &b, &a] {
        let (status, result) = call(
            &fixture.app,
            "PUT",
            "/api/editor/file",
            json!({"path":"notes/demo.txt","content":content}),
        )
        .await;
        if status != StatusCode::OK {
            stop.store(true, Ordering::Relaxed);
            reader.join().unwrap();
            panic!("atomic replacement failed: {status} {result}");
        }
    }
    stop.store(true, Ordering::Relaxed);
    reader.join().unwrap();
}
