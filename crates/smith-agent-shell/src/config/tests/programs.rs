use super::{parse_value, shaped_document};
use crate::config::{Program, ProgramName};

fn document() -> serde_json::Value {
    let mut document = shaped_document();
    document["programs"] = serde_json::json!({"claude_code": {
        "path": "/usr/local/bin/claude", "directory": "/home/u/.local/state/smith/programs"
    }});
    document
}

#[test]
fn a_configuration_names_claude_code_s_paths() {
    let configuration = parse_value(&document()).expect("program configuration");
    assert_eq!(configuration.programs.len(), 1);
    assert_eq!(
        configuration.programs.get(&ProgramName::ClaudeCode),
        Some(&Program {
            path: "/usr/local/bin/claude".into(),
            directory: "/home/u/.local/state/smith/programs".into(),
        })
    );
    assert!(configuration.endpoint_programs.is_empty(), "only endpoints naming a program are stored");
}

#[test]
fn an_endpoint_naming_a_program_reads_it() {
    let mut document = document();
    document["endpoints"][0]["program"] = "claude_code".into();
    let configuration = parse_value(&document).expect("endpoint program");
    assert_eq!(configuration.endpoint_programs.get(&0), Some(&ProgramName::ClaudeCode));
}

#[test]
fn an_endpoint_naming_a_program_without_a_path_is_refused() {
    let mut document = shaped_document();
    document["endpoints"][0]["program"] = "claude_code".into();
    let error = parse_value(&document).err().expect("undeclared program");
    assert!(error.contains("program[endpoint=0]"), "endpoint program field: {error}");
}

#[test]
fn a_relative_program_path_is_refused() {
    for field in ["path", "directory"] {
        let mut document = document();
        document["programs"]["claude_code"][field] = "relative".into();
        let error = parse_value(&document).err().expect("relative path");
        assert!(error.contains(&format!("programs.claude_code.{field}")), "path field: {error}");
    }
}

#[test]
fn an_unknown_program_is_refused() {
    let mut document = document();
    document["programs"]["other"] = document["programs"]["claude_code"].clone();
    let error = parse_value(&document).err().expect("unknown program");
    assert!(error.contains("programs.other"), "unknown field: {error}");
    let mut document = self::document();
    document["endpoints"][0]["program"] = "other".into();
    let error = parse_value(&document).err().expect("unknown endpoint program");
    assert!(error.contains("endpoints[0].program"), "endpoint field: {error}");
}

#[test]
fn program_paths_obey_the_byte_bound() {
    for field in ["path", "directory"] {
        for (path, accepted) in [
            (format!("/{}", "a".repeat(4095)), true),
            (format!("/{}", "a".repeat(4096)), false),
            ("/a\0b".into(), false),
        ] {
            let mut document = document();
            document["programs"]["claude_code"][field] = path.into();
            assert_eq!(parse_value(&document).is_ok(), accepted, "{field} bound");
        }
    }
}

#[test]
fn program_fields_are_required_strictly() {
    for field in ["path", "directory"] {
        let mut document = document();
        document["programs"]["claude_code"].as_object_mut().expect("program object").remove(field);
        let error = parse_value(&document).err().expect("required program field");
        assert!(error.contains(&format!("programs.claude_code.{field}")), "missing field: {error}");
    }
    let mut document = document();
    document["programs"]["claude_code"]["extra"] = true.into();
    let error = parse_value(&document).err().expect("unknown program field");
    assert!(error.contains("extra"), "unknown field: {error}");
}
