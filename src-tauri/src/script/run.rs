//! Running a generated script: the `ScriptRunner` seam and the only place
//! `powershell.exe` is spawned.
//!
//! Callers (`hardware::probe`, the switch command) take a `&dyn ScriptRunner` so tests can
//! supply [`FakeScriptRunner`] with canned output and never touch the machine.

use std::path::Path;
use std::process::Command;

use crate::error::AppError;

/// What a finished script left behind.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptOutput {
    /// The process exit code, or -1 when Windows reported none (killed).
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ScriptOutput {
    pub fn succeeded(&self) -> bool {
        self.exit_code == 0
    }
}

/// Runs a script file to completion.
pub trait ScriptRunner: Send + Sync {
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError>;
}

/// The real runner: Windows PowerShell 5.1 (ADR-0004), no profile, no prompts.
#[derive(Debug, Default, Clone)]
pub struct PowerShellRunner;

impl PowerShellRunner {
    pub const EXECUTABLE: &'static str = "powershell.exe";
}

impl ScriptRunner for PowerShellRunner {
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError> {
        let output = Command::new(Self::EXECUTABLE)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .args(args)
            .output()
            .map_err(|source| AppError::ScriptSpawn {
                path: script.to_path_buf(),
                source,
            })?;
        Ok(ScriptOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// A runner that returns canned output and records what it was asked to run.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct FakeScriptRunner {
    pub output: ScriptOutput,
    pub calls: std::sync::Mutex<Vec<(std::path::PathBuf, Vec<String>)>>,
}

#[cfg(test)]
impl FakeScriptRunner {
    pub fn returning(output: ScriptOutput) -> Self {
        FakeScriptRunner {
            output,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_stdout(stdout: &str) -> Self {
        Self::returning(ScriptOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }
}

#[cfg(test)]
impl ScriptRunner for FakeScriptRunner {
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((script.to_path_buf(), args.to_vec()));
        Ok(self.output.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn the_fake_returns_its_canned_output_and_records_the_call() {
        let runner: Box<dyn ScriptRunner> = Box::new(FakeScriptRunner::with_stdout("{}"));
        let output = runner
            .run(Path::new(r"C:\probe.ps1"), &["-Pause".to_string()])
            .unwrap();
        assert!(output.succeeded());
        assert_eq!(output.stdout, "{}");
    }

    #[test]
    fn the_fake_records_every_call_in_order() {
        let fake = FakeScriptRunner::default();
        fake.run(Path::new("a.ps1"), &[]).unwrap();
        fake.run(Path::new("b.ps1"), &["x".to_string()]).unwrap();
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].0, PathBuf::from("b.ps1"));
        assert_eq!(calls[1].1, vec!["x".to_string()]);
    }

    /// Windows PowerShell 5.1 ships with every Windows 10 and 11, so this is not a hardware
    /// test; it proves the spawn arguments, exit code and output capture on the real runner.
    #[test]
    fn the_real_runner_captures_stdout_and_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("hello.ps1");
        fs::write(
            &script,
            "param([string]$Name)\r\nWrite-Output \"hello $Name\"\r\nexit 3\r\n",
        )
        .unwrap();

        let output = PowerShellRunner
            .run(&script, &["-Name".to_string(), "layoutswap".to_string()])
            .unwrap();

        assert_eq!(output.exit_code, 3);
        assert_eq!(output.stdout.trim(), "hello layoutswap");
        assert!(!output.succeeded());
    }

    #[test]
    fn a_missing_script_is_a_failed_run_not_a_spawn_error() {
        let output = PowerShellRunner
            .run(Path::new(r"C:\does\not\exist.ps1"), &[])
            .unwrap();
        assert!(!output.succeeded());
    }
}
