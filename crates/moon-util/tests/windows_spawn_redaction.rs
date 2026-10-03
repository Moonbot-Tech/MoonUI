//! Windows spawn failures must not expose secrets from shell arguments.
#![cfg(windows)]

use std::process::{Command, Stdio};

/// Catches Debug escaping before redaction exposing the tail of an API_KEY
/// assignment in a shell argument when the executable cannot be started.
#[test]
fn windows_spawn_error_does_not_expose_a_shell_assignment_secret() {
    let mut command = Command::new("Z:\\moonui-nonexistent-bughunt-program.exe");
    command.args([
        "-c",
        r#"API_KEY="first moonui_bughunt_secret_sentinel" run-tool"#,
    ]);
    let error = match moon_util::process::Child::spawn(
        command,
        Stdio::null(),
        Stdio::null(),
        Stdio::null(),
    ) {
        Ok(_) => panic!("the deliberately nonexistent executable must fail to spawn"),
        Err(error) => format!("{error:#}"),
    };
    assert!(
        !error.contains("moonui_bughunt_secret_sentinel"),
        "the spawn error must redact API_KEY, got: {error}"
    );
}
