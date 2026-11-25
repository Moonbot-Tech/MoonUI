//! `ShellBuilder` wraps a task command so its stdin reads from the null device,
//! with one wrapper per shell family, in both `build` and `build_no_quote`.

use moon_util::shell::Shell;
use moon_util::shell_builder::ShellBuilder;

/// Runs `build_no_quote` for `program` with stdin redirected.
fn no_quote_redirected(program: &str, command: &str, args: &[&str]) -> (String, Vec<String>) {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    ShellBuilder::new(&Shell::Program(program.to_owned()), false)
        .redirect_stdin_to_dev_null()
        .build_no_quote(Some(command.to_owned()), &args)
}

/// Runs `build` for `program` with stdin redirected.
fn quoted_redirected(program: &str, command: &str, args: &[&str]) -> (String, Vec<String>) {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    ShellBuilder::new(&Shell::Program(program.to_owned()), false)
        .redirect_stdin_to_dev_null()
        .build(Some(command.to_owned()), &args)
}

/// Catches `build_no_quote` losing or changing a shell family's stdin
/// wrapper (or the `-i -c` / `-C` / `/S /C` launch arguments): a task started
/// through it would then wait on the terminal's stdin instead of reading EOF.
/// Arguments stay unquoted here, so `a b` reaches the shell as two words.
#[test]
fn build_no_quote_wraps_each_shell_family_for_a_null_stdin() {
    assert_eq!(
        no_quote_redirected("sh", "echo", &["a b"]),
        (
            "sh".to_owned(),
            vec![
                "-i".to_owned(),
                "-c".to_owned(),
                "exec </dev/null; echo a b".to_owned()
            ]
        )
    );
    assert_eq!(
        no_quote_redirected("nu", "echo", &["a b"]),
        (
            "nu".to_owned(),
            vec![
                "-i".to_owned(),
                "-c".to_owned(),
                "(echo a b\n) </dev/null".to_owned()
            ]
        )
    );
    assert_eq!(
        no_quote_redirected("powershell", "echo", &["a b"]),
        (
            "powershell".to_owned(),
            vec!["-C".to_owned(), "$null | & {echo a b}".to_owned()]
        )
    );
    assert_eq!(
        no_quote_redirected("cmd", "echo", &["a b"]),
        (
            "cmd".to_owned(),
            vec![
                "/S".to_owned(),
                "/C".to_owned(),
                "\"echo a b< NUL\"".to_owned()
            ]
        )
    );
}

/// Catches `build` losing or changing the PowerShell or cmd stdin wrapper,
/// the two families its existing unit tests never redirect: a task launched
/// in those shells would then block on the terminal's stdin.
#[test]
fn build_wraps_powershell_and_cmd_for_a_null_stdin() {
    assert_eq!(
        quoted_redirected("powershell", "echo", &["hi"]),
        (
            "powershell".to_owned(),
            vec!["-C".to_owned(), "$null | & {echo hi}".to_owned()]
        )
    );
    assert_eq!(
        quoted_redirected("cmd", "echo", &["hi"]),
        (
            "cmd".to_owned(),
            vec![
                "/S".to_owned(),
                "/C".to_owned(),
                "\"echo hi< NUL\"".to_owned()
            ]
        )
    );
}

/// Catches either builder applying the stdin wrapper (or appending launch
/// arguments) when there is no task command: the shell must then start with
/// only the arguments it was configured with.
#[test]
fn no_task_command_keeps_the_configured_arguments_only() {
    let shell = Shell::WithArguments {
        program: "sh".to_owned(),
        args: vec!["-l".to_owned()],
        title_override: None,
    };
    let expected = ("sh".to_owned(), vec!["-l".to_owned()]);
    assert_eq!(
        ShellBuilder::new(&shell, false)
            .redirect_stdin_to_dev_null()
            .build(None, &[]),
        expected
    );
    assert_eq!(
        ShellBuilder::new(&shell, false)
            .redirect_stdin_to_dev_null()
            .build_no_quote(None, &[]),
        expected
    );
}
