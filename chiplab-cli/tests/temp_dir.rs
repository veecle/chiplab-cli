//! Unit tests for the temporary directory redaction used by the fixture tests, which can't contain
//! them as they don't use the standard test harness.

#[path = "cli/temp_dir.rs"]
mod temp_dir;

#[test]
fn redacts_temporary_directories() {
    for (input, expected) in [
        ("tmp/foo-QHpLw0/test.ts", "[CHIPLAB_TEMP_DIR]/test.ts"),
        ("./tmp/foo-QHpLw0/test.ts", "[CHIPLAB_TEMP_DIR]/test.ts"),
        ("../tmp/foo-QHpLw0/test.ts", "[CHIPLAB_TEMP_DIR]/test.ts"),
        ("/tmp/foo-QHpLw0/test.ts", "[CHIPLAB_TEMP_DIR]/test.ts"),
        ("/var/tmp/foo-QHpLw0/test.ts", "[CHIPLAB_TEMP_DIR]/test.ts"),
        ("/tmp/foo-bar-QHpLw0", "[CHIPLAB_TEMP_DIR]"),
        ("tmp/foo.v2-QHpLw0/x", "[CHIPLAB_TEMP_DIR]/x"),
        ("/tmp/.tmpAbC123/x", "[CHIPLAB_TEMP_DIR]/x"),
        (
            "at f (./tmp/foo-QHpLw0/test.ts:3:5)",
            "at f ([CHIPLAB_TEMP_DIR]/test.ts:3:5)",
        ),
        (
            "Cannot find module 'tmp/foo-QHpLw0/x'.",
            "Cannot find module '[CHIPLAB_TEMP_DIR]/x'.",
        ),
        (
            "tmp/foo-QHpLw0/a.ts tmp/bar-AbC123/b.ts",
            "[CHIPLAB_TEMP_DIR]/a.ts [CHIPLAB_TEMP_DIR]/b.ts",
        ),
    ] {
        assert_eq!(temp_dir::redact(input), expected, "redacting {input:?}");
    }
}

#[test]
fn keeps_other_paths() {
    for input in [
        "tmp/test.ts",
        "tmp/foo-12345678/test.ts",
        "tmp/foo-QHp/test.ts",
        "tmp/foo.v2/test.ts",
        "tmp/QHpLw0/test.ts",
        "footmp/foo-QHpLw0/test.ts",
        "/tmp/.tmpAbC1234/x",
    ] {
        assert_eq!(temp_dir::redact(input), input, "redacting {input:?}");
    }
}
