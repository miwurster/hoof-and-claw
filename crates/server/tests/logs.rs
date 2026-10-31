//! The server's log output as the Operator reads it.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[test]
fn with_log_format_json_every_log_line_is_one_json_object() {
    let dir = tempfile::tempdir().unwrap();
    let mut server = Command::new(env!("CARGO_BIN_EXE_server"))
        .env("PORT", "0")
        .env("DATABASE_PATH", dir.path().join("game.db"))
        .env("LOG_FORMAT", "json")
        .env("RUST_LOG", "debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = server.stdout.take().unwrap();
    let mut lines = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        let listening = line.contains("listening on");
        lines.push(line);
        if listening {
            break;
        }
    }
    server.kill().unwrap();
    let stderr = std::io::read_to_string(server.stderr.take().unwrap()).unwrap();
    server.wait().unwrap();

    assert!(
        lines
            .last()
            .is_some_and(|line| line.contains("listening on")),
        "the server should log `listening on <address>`, but logged {lines:#?} and {stderr}"
    );
    assert!(
        stderr.is_empty(),
        "the server should log only to stdout, but wrote {stderr}"
    );
    for line in &lines {
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("log line {line:?} is not JSON: {error}"));
        assert!(value.is_object(), "log line {line:?} is not a JSON object");
    }
}
