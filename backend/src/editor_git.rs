use axum::http::StatusCode;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub(super) fn run(
    root: &Path,
    args: &[&str],
    limit: usize,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let failure = |code: &str| (StatusCode::SERVICE_UNAVAILABLE, code.to_string());
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("GIT_")
        {
            command.env_remove(key);
        }
    }
    let mut child = command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["--no-pager", "-C"])
        .arg(root)
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.pager=cat",
            "-c",
            "diff.external=",
        ])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            tracing::error!(%error, "editor Git spawn failed");
            failure("editor_git_unavailable")
        })?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(failure("editor_git_pipe_unavailable"));
    };
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(limit as u64 + 1).read_to_end(&mut bytes);
        let _ = sender.send(result.map(|_| bytes));
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    let result = 'output: loop {
        match receiver.try_recv() {
            Ok(Ok(bytes)) if bytes.len() <= limit => loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        break 'output if status.success() {
                            Ok(bytes)
                        } else {
                            Err(failure("editor_git_failed"))
                        }
                    }
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Ok(None) => break 'output Err(failure("editor_git_timeout")),
                    Err(error) => {
                        tracing::error!(%error, "editor Git wait failed");
                        break 'output Err(failure("editor_git_wait_failed"));
                    }
                }
            },
            Ok(Ok(_)) => break Err(failure("editor_git_output_limit")),
            Ok(Err(error)) => {
                tracing::error!(%error, "editor Git output read failed");
                break Err(failure("editor_git_read_failed"));
            }
            Err(mpsc::TryRecvError::Disconnected) => break Err(failure("editor_git_read_failed")),
            Err(mpsc::TryRecvError::Empty) if Instant::now() >= deadline => {
                break Err(failure("editor_git_timeout"))
            }
            Err(mpsc::TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    if result.is_err() {
        if let Err(error) = child.kill() {
            tracing::debug!(%error, "editor Git cancellation after terminal check");
        }
    }
    child.wait().map_err(|error| {
        tracing::error!(%error, "editor Git join failed");
        failure("editor_git_join_failed")
    })?;
    reader
        .join()
        .map_err(|_| failure("editor_git_reader_failed"))?;
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn git_output_overflow_and_failed_commands_are_explicit_errors() {
        let root = tempfile::tempdir().unwrap();
        super::run(root.path(), &["init"], 8192).unwrap();
        let overflow = super::run(root.path(), &["rev-parse", "--show-toplevel"], 1).unwrap_err();
        assert_eq!(overflow.1, "editor_git_output_limit");
        let failed = super::run(root.path(), &["context-agent-invalid-command"], 8192).unwrap_err();
        assert_eq!(failed.1, "editor_git_failed");
    }
}
