use std::fs;
use std::process::Command;

const SAMPLE: &str = r##"
schema_version = 1

[asset]
name = "cli-sample"
kind = "image"
width = 3
height = 2
default_frame_duration_ms = 250
loop = true

[[styles]]
id = "plain"
fg = "#FFFFFF"

[[frames]]
id = "frame-1"

[[frames.cells]]
x = 1
y = 0
ch = "*"
style = "plain"
"##;

#[test]
fn cli_exports_text_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = dir.path().join("sample.tanim.toml");
    let output = dir.path().join("sample.txt");

    fs::write(&input, SAMPLE).expect("write sample");

    let status = Command::new(env!("CARGO_BIN_EXE_terminal_animator"))
        .arg("--export")
        .arg("text")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("run terminal_animator");

    assert!(status.success());
    assert_eq!(fs::read_to_string(output).expect("read output"), " * \n   ");
}
