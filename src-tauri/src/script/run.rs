//! Running a generated script: the `ScriptRunner` seam and the only place
//! `powershell.exe` is spawned.
//!
//! Callers (`hardware::probe`, the switch in `app::switch`) take a `&dyn ScriptRunner`
//! so tests can supply [`FakeScriptRunner`] with canned output and never touch the
//! machine. Two ways to run: [`ScriptRunner::run`] waits for the script and returns
//! everything it printed; [`ScriptRunner::start`] hands back a [`RunningScript`] whose
//! lines arrive as the script prints them and which can be killed from another thread.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::error::AppError;

/// What a finished script left behind.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptOutput {
    /// The process exit code, or -1 when Windows reported none.
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ScriptOutput {
    pub fn succeeded(&self) -> bool {
        self.exit_code == 0
    }
}

/// The exit code a killed script reports: `Child::kill` ends it through
/// `TerminateProcess` with this code, so a cancelled switch never reads as applied.
pub const KILLED_EXIT_CODE: i32 = 1;

/// Kills a running script. Safe to call from any thread, and a no-op once it has exited.
pub type CancelHandle = Arc<dyn Fn() + Send + Sync>;

/// A script that has been started with [`ScriptRunner::start`].
pub trait RunningScript: Send {
    /// The next line the script printed, without its line ending, blocking until one
    /// arrives. `None` once the script has closed its output.
    fn next_line(&mut self) -> Option<String>;

    /// A handle that kills the script from another thread.
    fn cancel_handle(&self) -> CancelHandle;

    /// Waits for the script to exit and returns its exit code. Call it after
    /// `next_line` has returned `None`, so the wait is short.
    fn wait(self: Box<Self>) -> i32;
}

/// Runs a script file.
pub trait ScriptRunner: Send + Sync {
    /// Runs the script to completion and returns what it printed.
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError>;

    /// Starts the script and returns it while it runs. Standard output and standard
    /// error both arrive through [`RunningScript::next_line`], in arrival order.
    fn start(&self, script: &Path, args: &[String]) -> Result<Box<dyn RunningScript>, AppError>;
}

/// The real runner: Windows PowerShell 5.1 (ADR-0004), no profile, no prompts, and no
/// console window of its own, so a script run from the app never flashes one.
#[derive(Debug, Default, Clone)]
pub struct PowerShellRunner;

impl PowerShellRunner {
    pub const EXECUTABLE: &'static str = "powershell.exe";

    fn command(script: &Path, args: &[String]) -> Command {
        let mut command = Command::new(Self::EXECUTABLE);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .args(args)
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
    }
}

impl ScriptRunner for PowerShellRunner {
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError> {
        let output = Self::command(script, args)
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

    fn start(&self, script: &Path, args: &[String]) -> Result<Box<dyn RunningScript>, AppError> {
        let mut child = Self::command(script, args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| AppError::ScriptSpawn {
                path: script.to_path_buf(),
                source,
            })?;
        let (sender, lines) = mpsc::channel();
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        // Both streams are drained on their own threads: a pipe nobody reads fills up
        // and blocks the script.
        forward_lines(stdout, sender.clone());
        forward_lines(stderr, sender);
        Ok(Box::new(PowerShellProcess {
            lines,
            child: Arc::new(Mutex::new(child)),
        }))
    }
}

/// Reads `stream` line by line onto `sender` until it closes. A line that is not valid
/// UTF-8 is kept with replacement characters rather than ending the stream.
fn forward_lines(stream: impl Read + Send + 'static, sender: Sender<String>) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buffer);
                    let line = line.trim_end_matches(['\r', '\n']).to_string();
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

struct PowerShellProcess {
    lines: Receiver<String>,
    child: Arc<Mutex<Child>>,
}

impl RunningScript for PowerShellProcess {
    fn next_line(&mut self) -> Option<String> {
        self.lines.recv().ok()
    }

    fn cancel_handle(&self) -> CancelHandle {
        let child = Arc::clone(&self.child);
        Arc::new(move || {
            if let Ok(mut child) = child.lock() {
                // Killing an exited process is an error that means nothing here.
                let _ = child.kill();
            }
        })
    }

    fn wait(self: Box<Self>) -> i32 {
        let mut child = match self.child.lock() {
            Ok(child) => child,
            Err(poisoned) => poisoned.into_inner(),
        };
        child
            .wait()
            .ok()
            .and_then(|status| status.code())
            .unwrap_or(-1)
    }
}

/// A runner that returns canned output and records what it was asked to run. `run`
/// returns `output`; `start` streams `output.stdout` line by line, or the lines given
/// to [`FakeScriptRunner::streaming`], and can pause after a line until a test lets it
/// go on, so a cancel can land before or after a given step.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct FakeScriptRunner {
    output: Mutex<ScriptOutput>,
    pub calls: std::sync::Mutex<Vec<(std::path::PathBuf, Vec<String>)>>,
    stream: Option<(Vec<String>, i32)>,
    pause: Option<(usize, Arc<Gate>)>,
}

/// A gate a paused fake script waits at until the test releases it or kills the script.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct Gate {
    state: Mutex<GateState>,
    changed: std::sync::Condvar,
}

#[cfg(test)]
#[derive(Debug, Default, Clone, Copy)]
struct GateState {
    released: bool,
    killed: bool,
}

#[cfg(test)]
impl Gate {
    pub fn release(&self) {
        self.state.lock().unwrap().released = true;
        self.changed.notify_all();
    }

    fn kill(&self) {
        self.state.lock().unwrap().killed = true;
        self.changed.notify_all();
    }

    fn killed(&self) -> bool {
        self.state.lock().unwrap().killed
    }

    /// Blocks until released or killed; returns whether the script was killed.
    fn wait(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        while !state.released && !state.killed {
            state = self.changed.wait(state).unwrap();
        }
        state.killed
    }
}

#[cfg(test)]
impl FakeScriptRunner {
    pub fn returning(output: ScriptOutput) -> Self {
        FakeScriptRunner {
            output: Mutex::new(output),
            ..Default::default()
        }
    }

    /// What `run` returns from now on, for a test whose probe changes mid-way.
    pub fn set_stdout(&self, stdout: &str) {
        *self.output.lock().unwrap() = ScriptOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        };
    }

    pub fn with_stdout(stdout: &str) -> Self {
        Self::returning(ScriptOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }

    /// What `start` streams, and the exit code once every line is out.
    pub fn streaming(mut self, lines: &[&str], exit_code: i32) -> Self {
        self.stream = Some((lines.iter().map(|l| l.to_string()).collect(), exit_code));
        self
    }

    /// `start` pauses after delivering `lines` lines until the gate is released or the
    /// script is killed.
    pub fn pausing_after(mut self, lines: usize) -> (Self, Arc<Gate>) {
        let gate = Arc::new(Gate::default());
        self.pause = Some((lines, Arc::clone(&gate)));
        (self, gate)
    }
}

#[cfg(test)]
impl ScriptRunner for FakeScriptRunner {
    fn run(&self, script: &Path, args: &[String]) -> Result<ScriptOutput, AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((script.to_path_buf(), args.to_vec()));
        Ok(self.output.lock().unwrap().clone())
    }

    fn start(&self, script: &Path, args: &[String]) -> Result<Box<dyn RunningScript>, AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((script.to_path_buf(), args.to_vec()));
        let (lines, exit_code) = match &self.stream {
            Some((lines, exit_code)) => (lines.clone(), *exit_code),
            None => {
                let output = self.output.lock().unwrap();
                (
                    output.stdout.lines().map(str::to_string).collect(),
                    output.exit_code,
                )
            }
        };
        let (pause_after, gate) = match &self.pause {
            Some((after, gate)) => (Some(*after), Arc::clone(gate)),
            None => (None, Arc::new(Gate::default())),
        };
        Ok(Box::new(FakeRunningScript {
            lines: lines.into_iter().collect(),
            delivered: 0,
            exit_code,
            pause_after,
            gate,
        }))
    }
}

#[cfg(test)]
struct FakeRunningScript {
    lines: std::collections::VecDeque<String>,
    delivered: usize,
    exit_code: i32,
    pause_after: Option<usize>,
    gate: Arc<Gate>,
}

#[cfg(test)]
impl RunningScript for FakeRunningScript {
    fn next_line(&mut self) -> Option<String> {
        if self.pause_after == Some(self.delivered) && self.gate.wait() {
            return None;
        }
        if self.gate.killed() {
            return None;
        }
        let line = self.lines.pop_front()?;
        self.delivered += 1;
        Some(line)
    }

    fn cancel_handle(&self) -> CancelHandle {
        let gate = Arc::clone(&self.gate);
        Arc::new(move || gate.kill())
    }

    fn wait(self: Box<Self>) -> i32 {
        if self.gate.killed() {
            KILLED_EXIT_CODE
        } else {
            self.exit_code
        }
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
        fake.start(Path::new("b.ps1"), &["x".to_string()]).unwrap();
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].0, PathBuf::from("b.ps1"));
        assert_eq!(calls[1].1, vec!["x".to_string()]);
    }

    #[test]
    fn the_fake_streams_its_lines_then_reports_the_exit_code() {
        let fake = FakeScriptRunner::default().streaming(&["one", "two"], 2);
        let mut script = fake.start(Path::new("s.ps1"), &[]).unwrap();
        assert_eq!(script.next_line().as_deref(), Some("one"));
        assert_eq!(script.next_line().as_deref(), Some("two"));
        assert_eq!(script.next_line(), None);
        assert_eq!(script.wait(), 2);
    }

    #[test]
    fn without_a_stream_the_fake_streams_its_stdout() {
        let fake = FakeScriptRunner::with_stdout("a\nb\n");
        let mut script = fake.start(Path::new("s.ps1"), &[]).unwrap();
        assert_eq!(script.next_line().as_deref(), Some("a"));
        assert_eq!(script.next_line().as_deref(), Some("b"));
        assert_eq!(script.next_line(), None);
        assert_eq!(script.wait(), 0);
    }

    #[test]
    fn a_paused_fake_waits_for_the_gate_and_a_killed_one_stops_short() {
        let (fake, gate) = FakeScriptRunner::default()
            .streaming(&["one", "two", "three"], 0)
            .pausing_after(1);
        let mut script = fake.start(Path::new("s.ps1"), &[]).unwrap();
        assert_eq!(script.next_line().as_deref(), Some("one"));
        gate.release();
        assert_eq!(script.next_line().as_deref(), Some("two"));

        let (fake, _gate) = FakeScriptRunner::default()
            .streaming(&["one", "two"], 0)
            .pausing_after(1);
        let mut script = fake.start(Path::new("s.ps1"), &[]).unwrap();
        assert_eq!(script.next_line().as_deref(), Some("one"));
        let cancel = script.cancel_handle();
        let reader = thread::spawn(move || {
            let rest = script.next_line();
            (rest, script.wait())
        });
        cancel();
        let (rest, exit_code) = reader.join().unwrap();
        assert_eq!(rest, None);
        assert_eq!(exit_code, KILLED_EXIT_CODE);
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
    fn the_real_runner_streams_lines_as_they_are_printed_and_can_be_killed() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("slow.ps1");
        fs::write(
            &script,
            "Write-Host 'first'\r\n[Console]::Error.WriteLine('from stderr')\r\nStart-Sleep -Seconds 30\r\nWrite-Host 'never'\r\nexit 0\r\n",
        )
        .unwrap();

        let mut running = PowerShellRunner.start(&script, &[]).unwrap();
        let mut seen = Vec::new();
        while seen.len() < 2 {
            seen.push(running.next_line().expect("two lines before the sleep"));
        }
        seen.sort();
        assert_eq!(seen, vec!["first".to_string(), "from stderr".to_string()]);

        let cancel = running.cancel_handle();
        cancel();
        assert_eq!(running.next_line(), None, "the output closes on kill");
        assert_eq!(running.wait(), KILLED_EXIT_CODE);
    }

    #[test]
    fn a_missing_script_is_a_failed_run_not_a_spawn_error() {
        let output = PowerShellRunner
            .run(Path::new(r"C:\does\not\exist.ps1"), &[])
            .unwrap();
        assert!(!output.succeeded());
    }
}
