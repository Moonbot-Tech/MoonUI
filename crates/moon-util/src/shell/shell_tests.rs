use super::ShellKind;

/// Catches `ShellKind::new` sending an unknown program to POSIX on Windows,
/// so a task would look for `sh` and fail to start a shell.
#[test]
fn unknown_program_falls_back_to_powershell_only_on_windows() {
    assert_eq!(ShellKind::new("cmd.exe", false), ShellKind::Cmd);
    assert_eq!(
        ShellKind::new("powershell.exe", true),
        ShellKind::PowerShell
    );
    assert_eq!(ShellKind::new("pwsh", false), ShellKind::Pwsh);
    assert_eq!(ShellKind::new("bash", false), ShellKind::Posix);
    assert_eq!(ShellKind::new("fancy", true), ShellKind::PowerShell);
    assert_eq!(ShellKind::new("fancy", false), ShellKind::Posix);
}

/// Catches `to_cmd_variable` rewriting `${VAR:-default}` into a percent name,
/// so cmd would look up `VAR:-default` instead of keeping the default text.
#[test]
fn cmd_rewrites_variables_but_leaves_default_substitutions() {
    assert_eq!(ShellKind::Cmd.to_shell_variable("$FOO"), "%FOO%");
    assert_eq!(ShellKind::Cmd.to_shell_variable("${FOO}"), "%FOO%");
    assert_eq!(
        ShellKind::Cmd.to_shell_variable("${FOO:-bar}"),
        "${FOO:-bar}"
    );
    assert_eq!(ShellKind::Cmd.to_shell_variable("plain"), "plain");
}

/// Catches `to_powershell_variable` leaving `$FOO` as a local variable, so a
/// PowerShell task would miss the environment value the caller set.
#[test]
fn powershell_rewrites_variables_onto_the_env_drive() {
    assert_eq!(ShellKind::PowerShell.to_shell_variable("$FOO"), "$env:FOO");
    assert_eq!(ShellKind::Pwsh.to_shell_variable("${BAR}"), "$env:BAR");
    assert_eq!(
        ShellKind::PowerShell.to_shell_variable("${FOO:-bar}"),
        "${FOO:-bar}"
    );
    assert_eq!(ShellKind::Pwsh.to_shell_variable("plain"), "plain");
}

/// Catches `args_for_shell` dropping the quotes around a cmd `/C` payload, so
/// cmd would split `echo hi` on the space and run a different command.
#[test]
fn cmd_args_quote_the_command_and_ignore_interactive() {
    let command = "echo hi".to_owned();
    let expected = vec!["/S".to_owned(), "/C".to_owned(), "\"echo hi\"".to_owned()];
    assert_eq!(
        ShellKind::Cmd.args_for_shell(true, command.clone()),
        expected
    );
    assert_eq!(ShellKind::Cmd.args_for_shell(false, command), expected);
}

/// Catches `args_for_shell` passing `-c` or `-i` to Windows PowerShell, so
/// the process would reject the flag and the task would not run.
#[test]
fn powershell_args_use_capital_c_without_an_interactive_flag() {
    let command = "echo hi".to_owned();
    let expected = vec!["-C".to_owned(), "echo hi".to_owned()];
    assert_eq!(
        ShellKind::PowerShell.args_for_shell(true, command.clone()),
        expected
    );
    assert_eq!(ShellKind::Pwsh.args_for_shell(false, command), expected);
}

/// Catches `args_for_shell` always passing `-i` to a POSIX shell, so a
/// non-interactive task would start an interactive shell and wait on stdin.
#[test]
fn posix_args_add_interactive_only_when_requested() {
    assert_eq!(
        ShellKind::Posix.args_for_shell(false, "echo hi".to_owned()),
        vec!["-c".to_owned(), "echo hi".to_owned()]
    );
    assert_eq!(
        ShellKind::Fish.args_for_shell(true, "echo hi".to_owned()),
        vec!["-i".to_owned(), "-c".to_owned(), "echo hi".to_owned()]
    );
}

/// Catches `parse_nushell_var` rewriting an empty `${}` into `$env.`, so a
/// task argument that was an empty expansion becomes the env drive itself.
#[test]
fn nushell_keeps_an_empty_brace_expansion() {
    assert_eq!(ShellKind::Nushell.to_shell_variable("${}"), "${}");
    assert_eq!(
        ShellKind::Nushell.to_shell_variable("pre${}post"),
        "pre${}post"
    );
}

/// Catches `parse_nushell_var` treating `$env.FOO` as a new name `env`, so a
/// value that is already an env-drive path is rewritten to `$env.env.FOO`.
#[test]
fn nushell_keeps_an_existing_env_drive() {
    assert_eq!(ShellKind::Nushell.to_shell_variable("$env.FOO"), "$env.FOO");
    assert_eq!(
        ShellKind::Nushell.to_shell_variable("pre$env.FOO"),
        "pre$env.FOO"
    );
}

/// Catches `parse_nushell_var` accepting a digit as a variable name, so `$1`
/// becomes `$env.1` and a positional parameter is read as an environment value.
#[test]
fn nushell_leaves_a_non_identifier_dollar_alone() {
    assert_eq!(ShellKind::Nushell.to_shell_variable("$1"), "$1");
    assert_eq!(ShellKind::Nushell.to_shell_variable("pre$1"), "pre$1");
}

/// Catches `prepend_command_prefix` stacking a second prefix onto a command
/// that already has one, so PowerShell runs `&&Get-ChildItem` and Nushell
/// runs `^^echo`.
#[test]
fn command_prefix_is_applied_once() {
    assert_eq!(
        ShellKind::PowerShell
            .prepend_command_prefix("Get-ChildItem")
            .as_ref(),
        "&Get-ChildItem"
    );
    assert_eq!(
        ShellKind::PowerShell
            .prepend_command_prefix("&Get-ChildItem")
            .as_ref(),
        "&Get-ChildItem"
    );
    assert_eq!(
        ShellKind::Pwsh
            .prepend_command_prefix("Get-ChildItem")
            .as_ref(),
        "&Get-ChildItem"
    );
    assert_eq!(
        ShellKind::Nushell.prepend_command_prefix("echo").as_ref(),
        "^echo"
    );
    assert_eq!(
        ShellKind::Nushell.prepend_command_prefix("^echo").as_ref(),
        "^echo"
    );
    assert_eq!(
        ShellKind::Posix.prepend_command_prefix("echo").as_ref(),
        "echo"
    );
}

/// Catches `try_quote_prefix_aware` omitting `&` when PowerShell quoting
/// changes the command, so a program path with a space runs as a string
/// instead of as a command.
#[test]
fn powershell_prefix_aware_quote_prefixes_only_a_quoted_command() {
    let shell = ShellKind::PowerShell;
    assert_eq!(
        shell.try_quote_prefix_aware("echo").unwrap().as_ref(),
        "echo"
    );
    assert_eq!(
        shell.try_quote_prefix_aware("echo hi").unwrap().as_ref(),
        "&'echo hi'"
    );
    assert_eq!(
        shell.try_quote_prefix_aware("&echo").unwrap().as_ref(),
        "&echo"
    );
    assert_eq!(
        shell.try_quote_prefix_aware("&'echo hi'").unwrap().as_ref(),
        "&'echo hi'"
    );
}
