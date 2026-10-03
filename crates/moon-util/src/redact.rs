use std::sync::LazyLock;

static REDACT_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"([A-Z_][A-Z0-9_]*)=("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|\S+)"#).unwrap()
});

/// Whether a given environment variable name should have its value redacted
pub fn should_redact(env_var_name: &str) -> bool {
    const REDACTED_SUFFIXES: &[&str] = &[
        "KEY",
        "TOKEN",
        "PASSWORD",
        "SECRET",
        "PASS",
        "CREDENTIALS",
        "LICENSE",
    ];
    REDACTED_SUFFIXES
        .iter()
        .any(|suffix| env_var_name.ends_with(suffix))
}

/// Redact a string which could include a command with environment variables
pub fn redact_command(command: &str) -> String {
    REDACT_REGEX
        .replace_all(command, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let value = &caps[2];
            if should_redact(var_name) {
                format!(r#"{}="[REDACTED]""#, var_name)
            } else {
                format!("{}={}", var_name, value)
            }
        })
        .to_string()
}

/// Formats a command's program and arguments for diagnostics, redacting each
/// raw value before Debug quoting can obscure assignment boundaries. Explicit
/// environment overrides are omitted so their values cannot enter the context.
pub(crate) fn format_redacted_command(command: &std::process::Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|value| format!("{:?}", redact_command(&value.to_string_lossy())))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod redact_tests;
