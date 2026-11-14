//! End-to-end CLI integration tests for the `cius-ro` binary.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_cius-ro")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("cius-ro/tests/fixtures")
}

#[test]
fn cli_check_compliant_xml_and_pdf_exit_zero() {
    if !fixtures_dir().exists() {
        return;
    }
    let xml = fixtures_dir().join("01_standard_b2b_ro_ubl.xml");
    let pdf = fixtures_dir().join("01_standard_b2b_ro_ubl.pdf");

    let out = Command::new(bin_path())
        .args(["check", "--lang", "ro", "--deny-warnings"])
        .arg(&xml)
        .arg(&pdf)
        .output()
        .expect("spawn cius-ro check");

    assert!(
        out.status.success(),
        "Expected exit 0, got {:?}\nstdout: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[VALID]"));
}

#[test]
fn cli_check_invalid_invoice_exits_one_in_json_svrl_and_sarif() {
    if !fixtures_dir().exists() {
        return;
    }
    let faulty = fixtures_dir().join("faulty_08_ro_invalid_sector_cui.xml");

    for fmt in ["json", "svrl", "sarif"] {
        let out = Command::new(bin_path())
            .args(["check", "--format", fmt, "--lang", "ro"])
            .arg(&faulty)
            .output()
            .expect("spawn cius-ro check");

        assert_eq!(
            out.status.code(),
            Some(1),
            "Expected exit code 1 for invalid invoice ({fmt})"
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        match fmt {
            "json" => {
                let v: serde_json::Value =
                    serde_json::from_str(&stdout).expect("valid JSON report");
                assert_eq!(v["isValid"], false);
            }
            "svrl" => {
                assert!(stdout.contains("<svrl:schematron-output"));
                assert!(stdout.contains("BR-RO-110"));
            }
            "sarif" => {
                let v: serde_json::Value =
                    serde_json::from_str(&stdout).expect("valid SARIF v2.1.0 JSON");
                assert_eq!(v["version"], "2.1.0");
                assert!(!v["runs"][0]["results"].as_array().unwrap().is_empty());
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn cli_check_missing_file_exits_two_and_stdin_pipe_works() {
    let missing = Command::new(bin_path())
        .args(["check", "/nonexistent/cius_ro_missing_12345.xml"])
        .output()
        .expect("spawn cius-ro");
    assert_eq!(missing.status.code(), Some(2));

    // Pipe valid XML through stdin (`-`)
    if !fixtures_dir().exists() {
        return;
    }
    let xml_bytes = std::fs::read(fixtures_dir().join("01_standard_b2b_ro_ubl.xml")).unwrap();
    let mut child = Command::new(bin_path())
        .args(["check", "-", "--format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn stdin check");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&xml_bytes)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait stdin check");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["isValid"], true);
}

#[test]
fn cli_extract_id_and_rules_subcommands() {
    // 1. extract PDF -> stdout XML
    if fixtures_dir().exists() {
        let pdf = fixtures_dir().join("01_standard_b2b_ro_ubl.pdf");
        let ext = Command::new(bin_path())
            .arg("extract")
            .arg(&pdf)
            .output()
            .expect("spawn extract");
        assert!(ext.status.success());
        let xml = String::from_utf8(ext.stdout).expect("UTF-8 extracted XML");
        assert!(xml.contains("EFACT-RO-2026-0042"));
    }

    // 2. id subcommand (valid CUI vs invalid fuzzy Bucharest sector)
    let id_ok = Command::new(bin_path())
        .args(["id", "RO45678918", "--json", "--lang", "ro"])
        .output()
        .expect("spawn id valid");
    assert!(id_ok.status.success());
    let id_json: serde_json::Value = serde_json::from_slice(&id_ok.stdout).unwrap();
    assert_eq!(id_json["isValid"], true);
    assert_eq!(id_json["kind"], "cui");

    let id_fuzzy = Command::new(bin_path())
        .args(["id", "Sector 3", "--json", "--lang", "ro"])
        .output()
        .expect("spawn id fuzzy");
    assert_eq!(id_fuzzy.status.code(), Some(1));
    let fuzzy_json: serde_json::Value = serde_json::from_slice(&id_fuzzy.stdout).unwrap();
    assert_eq!(fuzzy_json["normalized"], "SECTOR3");

    // 3. rules subcommand
    let rule_out = Command::new(bin_path())
        .args(["rules", "BR-RO-Z2", "--json", "--lang", "ro"])
        .output()
        .expect("spawn rules");
    assert!(rule_out.status.success());
    let rule_json: serde_json::Value = serde_json::from_slice(&rule_out.stdout).unwrap();
    assert_eq!(rule_json["code"], "BR-RO-Z2");
}
