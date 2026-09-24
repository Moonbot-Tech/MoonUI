use std::process::ExitStatus;

use super::{parse_env_map_from_noisy_output, parse_env_output};

/// Builds an `ExitStatus` whose code is `code` on the host platform.
#[cfg(unix)]
fn exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    ExitStatus::from_raw(code << 8)
}

/// Builds an `ExitStatus` whose code is `code` on the host platform.
#[cfg(windows)]
fn exit_status(code: u32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    ExitStatus::from_raw(code)
}

/// Catches `parse_env_map_from_noisy_output` stopping at the first `{`, so a
/// shell banner that contains a brace hides the environment JSON after it.
#[test]
fn later_json_object_is_used_when_an_earlier_brace_is_not_a_map() {
    let output = "banner {not-json} tail {\"PATH\":\"/usr/bin\",\"HOME\":\"/home/dev\"} done";

    let env_map = parse_env_map_from_noisy_output(output)
        .expect("the object after the banner brace should parse");

    assert_eq!(env_map.get("PATH").map(String::as_str), Some("/usr/bin"));
    assert_eq!(env_map.get("HOME").map(String::as_str), Some("/home/dev"));
    assert_eq!(env_map.len(), 2);
}

/// Catches `parse_env_map_from_noisy_output` reporting success when the shell
/// printed no JSON, so a failed capture looks like an empty environment.
#[test]
fn missing_json_is_an_error() {
    let output = "shell ready, no environment";

    let error = parse_env_map_from_noisy_output(output)
        .expect_err("output with no JSON object should be an error");

    let message = error.to_string();
    assert!(
        message.contains("Failed to find JSON in shell output"),
        "{message}"
    );
    assert!(message.contains(output), "{message}");
}

/// Catches `parse_env_output` treating a zero exit as a failure after the
/// environment JSON parsed, so a healthy login shell is discarded.
#[test]
fn successful_exit_returns_parsed_env_without_either_callback() {
    let env_map = parse_env_output(
        "noise\n{\"PATH\":\"/usr/bin\"}\n",
        &exit_status(0),
        || panic!("warning should not run when the shell exited 0"),
        || panic!("failure message should not run when the JSON parsed"),
    )
    .expect("parsed environment JSON should be returned on a zero exit");

    assert_eq!(env_map.get("PATH").map(String::as_str), Some("/usr/bin"));
    assert_eq!(env_map.len(), 1);
}

/// Catches `parse_env_output` dropping the shell's own error when the output
/// is not JSON and the process exited non-zero, so the caller only sees a
/// parse failure.
#[test]
fn failed_exit_with_unparseable_output_includes_the_shell_error() {
    let error = parse_env_output(
        "not json",
        &exit_status(1),
        || panic!("warning is only for a parsed environment"),
        || "shell died".to_string(),
    )
    .expect_err("unparseable output should be an error");

    let message = error.to_string();
    assert!(message.contains("shell died"), "{message}");
    assert!(
        message.contains("Failed to deserialize environment variables from json"),
        "{message}"
    );
    assert!(message.contains("not json"), "{message}");
}

/// Catches `parse_env_output` blaming a shell crash when the process exited
/// zero and the output was not JSON, so a successful command is reported as
/// a failure of the shell itself.
#[test]
fn successful_exit_with_unparseable_output_omits_the_shell_error() {
    let error = parse_env_output(
        "not json",
        &exit_status(0),
        || panic!("warning is only for a parsed environment"),
        || "shell died".to_string(),
    )
    .expect_err("unparseable output should be an error");

    let message = error.to_string();
    assert!(
        !message.contains("shell died"),
        "zero exit should not include the shell-failure text: {message}"
    );
    assert!(
        message.contains("Failed to deserialize environment variables from json"),
        "{message}"
    );
    assert!(message.contains("not json"), "{message}");
}
