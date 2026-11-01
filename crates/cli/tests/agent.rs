use serde_json::Value;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_project-knowledge"))
}

#[test]
fn agent_cli_configuration_is_reachable_in_both_argument_orders() {
    let temp = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["init", "--root"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let config = temp.path().join(".project-knowledge/project.json");
    let env = temp.path().join(".env");
    std::fs::write(&env,"PROJECT_KNOWLEDGE_PROVIDER=openai-compatible\nPROJECT_KNOWLEDGE_BASE_URL=http://127.0.0.1:1/v1\nPROJECT_KNOWLEDGE_MODEL='cli-model'\nCLI_TEST_KEY=private-cli-value\n").unwrap();
    #[cfg(feature = "agent")]
    {
        for before in [true, false] {
            let mut cmd = cli();
            cmd.arg("agent");
            if before {
                cmd.arg("--config").arg(&config);
            }
            cmd.arg("configure");
            if !before {
                cmd.arg("--config").arg(&config);
            }
            cmd.arg("--env-file")
                .arg(&env)
                .args(["--api-key-env", "CLI_TEST_KEY"]);
            let output = cmd.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["status"], "configured");
            assert_eq!(value["agent"]["model"], "cli-model");
            assert_eq!(value["keyConfigured"], true);
            assert!(!String::from_utf8_lossy(&output.stdout).contains("private-cli-value"));
            assert!(
                !std::fs::read_to_string(&config)
                    .unwrap()
                    .contains("private-cli-value")
            );
        }
        let output = cli()
            .args(["agent", "build"])
            .arg("--config")
            .arg(&config)
            .arg("--env-file")
            .arg(&env)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], "INDEX_MISSING");
    }
    #[cfg(not(feature = "agent"))]
    {
        let output = cli()
            .args(["agent", "configure", "--config"])
            .arg(&config)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], "NOT_IMPLEMENTED");
    }
}
