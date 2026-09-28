use super::redact_command;

/// Catches the single-quoted alternative disappearing from the assignment
/// regex, so a password whose value contains a space survives into the spawn
/// error that `process.rs` logs.
#[test]
fn single_quoted_password_with_a_space_is_redacted() {
    let input = "start DB_PASSWORD='secret value' tail";
    let redacted = redact_command(input);
    assert_eq!(
        redacted, "start DB_PASSWORD=\"[REDACTED]\" tail",
        "a quoted password must be replaced as a whole, not cut at the space"
    );
    assert!(
        !redacted.contains("secret value"),
        "the password text must not remain in the log line"
    );
}

/// Catches the unquoted-value alternative disappearing from the assignment
/// regex, so a token written without quotes is copied verbatim into the
/// spawn error.
#[test]
fn unquoted_token_is_redacted() {
    let input = "export GITHUB_TOKEN=ghp_abc123 && true";
    let redacted = redact_command(input);
    assert_eq!(
        redacted, "export GITHUB_TOKEN=\"[REDACTED]\" && true",
        "an unquoted token is still an assignment and must be redacted"
    );
    assert!(
        !redacted.contains("ghp_abc123"),
        "the token text must not remain in the log line"
    );
}

/// Catches `should_redact` switching from a suffix check to a substring
/// check, so a name that merely contains `SECRET` (such as `SECRETARY`)
/// would be wiped while a real secret must still be.
#[test]
fn secret_suffix_redacts_and_a_containing_name_does_not() {
    let input = r#"AWS_SECRET="top" SECRETARY="kept" AWS_REGION="us-east-1""#;
    assert_eq!(
        redact_command(input),
        r#"AWS_SECRET="[REDACTED]" SECRETARY="kept" AWS_REGION="us-east-1""#,
        "only a name that ends with a secret suffix is redacted"
    );
}

/// Catches the double-quoted value pattern ignoring backslash escapes, so
/// an embedded quote would end the value early and the rest of the command
/// would be logged with the secret still in it.
#[test]
fn escaped_quote_inside_a_key_does_not_leak_the_remainder() {
    let input = r#"API_KEY="a\"b c" HOME="/Users/foo""#;
    let redacted = redact_command(input);
    assert_eq!(
        redacted, r#"API_KEY="[REDACTED]" HOME="/Users/foo""#,
        "the escaped quote is inside the value, and HOME is a separate assignment"
    );
    assert!(
        !redacted.contains("b c"),
        "the tail of the secret value must not remain after the escaped quote"
    );
}

/// Catches `CREDENTIALS` being dropped from the suffix list, so a cloud
/// credentials assignment would be written into the spawn error.
#[test]
fn credentials_suffix_is_redacted() {
    let input = r#"CLOUD_CREDENTIALS="abc" PATH="/usr/bin""#;
    assert_eq!(
        redact_command(input),
        r#"CLOUD_CREDENTIALS="[REDACTED]" PATH="/usr/bin""#,
        "CREDENTIALS is a secret suffix and PATH is not"
    );
}

/// Catches `PASS` being dropped from the suffix list. `PASSWORD` still ends
/// with `PASS`, so a shorter name such as `VNC_PASS` is the one that goes
/// unprotected.
#[test]
fn pass_suffix_is_redacted_without_taking_the_password_word() {
    let input = r#"VNC_PASS="abc" PORT="kept""#;
    assert_eq!(
        redact_command(input),
        r#"VNC_PASS="[REDACTED]" PORT="kept""#,
        "VNC_PASS ends with PASS; PORT does not"
    );
}
