use std::fs;

use super::patch_sum_tree_tracing;

/// Catches dropping either LF or CRLF support while removing GPL tracing hooks from extracted sources.
#[test]
fn sum_tree_tracing_patch_handles_lf_and_crlf_sources() {
    for newline in ["\n", "\r\n"] {
        let temp_dir = tempfile::tempdir().expect("create patch fixture");
        let source_dir = temp_dir.path().join("src");
        fs::create_dir_all(&source_dir).expect("create source fixture");
        let source = [
            "use ztracing::instrument;",
            "",
            "impl Cursor {",
            "    #[instrument(skip_all)]",
            "    fn visit(&self) {}",
            "",
            "    #[ctor::ctor(unsafe)]",
            "    fn init_logger() {",
            "        zlog::init_test();",
            "    }",
            "",
            "",
            "    #[test]",
            "    fn visits() {}",
            "}",
            "",
        ]
        .join(newline);
        let source_path = source_dir.join("cursor.rs");
        fs::write(&source_path, source).expect("write patch fixture");

        patch_sum_tree_tracing(temp_dir.path()).expect("patch tracing fixture");

        let patched = fs::read_to_string(&source_path).expect("read patched fixture");
        assert!(!patched.contains("ztracing"));
        assert!(!patched.contains("#[instrument"));
        assert!(!patched.contains("#[ctor::ctor"));
        assert!(!patched.contains("zlog::init_test"));
        assert!(patched.contains("fn visit(&self) {}"));
        assert!(patched.contains("#[test]"));
        if newline == "\r\n" {
            assert!(!patched.replace("\r\n", "").contains('\n'));
        } else {
            assert!(!patched.contains('\r'));
        }
    }
}
