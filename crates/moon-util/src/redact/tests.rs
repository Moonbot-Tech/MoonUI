//! Redaction and diagnostic formatting regressions.

use super::{format_redacted_command, redact_command};
use std::process::Command;

/// Catches losing one of several sensitive assignments while retaining useful
/// non-sensitive environment assignments in the diagnostic.
#[test]
fn test_redact_string_with_multiple_env_vars() {
    let input = r#"failed to spawn command cd "/code/something" && ANTHROPIC_API_KEY="sk-ant-api03-WOOOO" COMMAND_MODE="unix2003" GEMINI_API_KEY="AIGEMINIFACE" HOME="/Users/foo""#;
    let result = redact_command(input);
    let expected = r#"failed to spawn command cd "/code/something" && ANTHROPIC_API_KEY="[REDACTED]" COMMAND_MODE="unix2003" GEMINI_API_KEY="[REDACTED]" HOME="/Users/foo""#;
    assert_eq!(result, expected);
}

/// Catches applying Debug escaping before redaction, which leaks secret tails,
/// or combining arguments before redaction, which obscures argument boundaries.
#[test]
fn command_diagnostics_redact_raw_assignments_before_quoting() {
    for assignment in [
        r#"API_KEY="first sentinel tail" run-tool"#,
        r#"API_KEY="first\"sentinel tail" run-tool"#,
        "API_KEY='first sentinel tail' run-tool",
        "API_KEY=sentinel run-tool",
    ] {
        let mut command = Command::new("tool");
        command.args([
            "-c",
            assignment,
            r#"HOME="/tmp/a b""#,
            r#"ordinary "quotes""#,
            "",
        ]);
        assert_eq!(
            format_redacted_command(&command),
            r#""tool" "-c" "API_KEY=\"[REDACTED]\" run-tool" "HOME=\"/tmp/a b\"" "ordinary \"quotes\"" """#,
            "the entire secret must disappear while other arguments stay readable"
        );
    }
}

/// Catches leaving the program unredacted or including environment overrides,
/// either of which can expose a secret even when arguments are redacted.
#[test]
fn command_diagnostics_redact_the_program_and_omit_environment_values() {
    let mut command = Command::new(r#"API_KEY="program sentinel""#);
    command
        .arg("--help")
        .env("API_TOKEN", "environment sentinel");
    assert_eq!(
        format_redacted_command(&command),
        r#""API_KEY=\"[REDACTED]\"" "--help""#
    );
}
