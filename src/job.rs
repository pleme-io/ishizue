//! Async job runner — spawn external processes and capture output.
//!
//! Pure Rust implementation using [`std::process::Command`]. No nvim-oxi
//! dependency — results are plain strings, easily forwarded to Neovim.
//!
//! # Examples
//!
//! ```
//! use ishizue::job::Job;
//!
//! let handle = Job::new("echo", &["hello"])
//!     .spawn()
//!     .expect("failed to spawn");
//! let output = handle.wait().expect("failed to wait");
//! assert!(output.success);
//! assert_eq!(output.stdout.trim(), "hello");
//! ```

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Builder for configuring and spawning an external process.
#[derive(Debug, Clone)]
pub struct Job {
    cmd: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: HashMap<String, String>,
    env_clear: bool,
    stdin_data: Option<String>,
}

impl Job {
    /// Create a new job that will run `cmd` with the given `args`.
    #[must_use]
    pub fn new(cmd: &str, args: &[&str]) -> Self {
        Self {
            cmd: cmd.to_owned(),
            args: args.iter().map(|&s| s.to_owned()).collect(),
            cwd: None,
            env: HashMap::new(),
            env_clear: false,
            stdin_data: None,
        }
    }

    /// Set the working directory for the spawned process.
    #[must_use]
    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Set an environment variable for the spawned process.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Clear all inherited environment variables before applying those set
    /// via [`Job::env`].
    #[must_use]
    pub fn env_clear(mut self) -> Self {
        self.env_clear = true;
        self
    }

    /// Provide data to write to the child's stdin after spawning.
    #[must_use]
    pub fn stdin(mut self, data: impl Into<String>) -> Self {
        self.stdin_data = Some(data.into());
        self
    }

    /// Spawn the process and return a [`JobHandle`] for awaiting its result.
    pub fn spawn(&self) -> io::Result<JobHandle> {
        let mut command = Command::new(&self.cmd);
        command.args(&self.args);

        if let Some(ref cwd) = self.cwd {
            command.current_dir(cwd);
        }

        if self.env_clear {
            command.env_clear();
        }

        for (k, v) in &self.env {
            command.env(k, v);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if self.stdin_data.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        let mut child = command.spawn()?;

        // Write stdin data if provided.
        if let Some(ref data) = self.stdin_data {
            use std::io::Write;
            if let Some(mut stdin_pipe) = child.stdin.take() {
                stdin_pipe.write_all(data.as_bytes())?;
                // Drop closes the pipe, signaling EOF to the child.
            }
        }

        Ok(JobHandle { child })
    }
}

/// Handle to a running child process. Use [`JobHandle::wait`] to block until
/// completion and collect output.
#[derive(Debug)]
pub struct JobHandle {
    child: Child,
}

impl JobHandle {
    /// Return the OS-assigned process ID.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Send `SIGKILL` to the child process.
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    /// Block until the child exits and collect its output.
    pub fn wait(self) -> io::Result<JobOutput> {
        let output = self.child.wait_with_output()?;
        Ok(JobOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Collected output from a completed process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutput {
    /// `true` when the process exited with status 0.
    pub success: bool,
    /// The exit code, or `None` if the process was killed by a signal.
    pub exit_code: Option<i32>,
    /// Everything written to stdout, decoded as lossy UTF-8.
    pub stdout: String,
    /// Everything written to stderr, decoded as lossy UTF-8.
    pub stderr: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_echo() {
        let handle = Job::new("echo", &["hello", "world"])
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout.trim(), "hello world");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn spawn_with_cwd() {
        let handle = Job::new("pwd", &[])
            .cwd("/tmp")
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        // macOS resolves /tmp -> /private/tmp
        assert!(
            output.stdout.trim() == "/tmp" || output.stdout.trim() == "/private/tmp",
            "unexpected pwd: {}",
            output.stdout.trim(),
        );
    }

    #[test]
    fn spawn_with_env() {
        let handle = Job::new("sh", &["-c", "echo $MY_TEST_VAR"])
            .env("MY_TEST_VAR", "ishizue")
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        assert_eq!(output.stdout.trim(), "ishizue");
    }

    #[test]
    fn spawn_with_env_clear() {
        let handle = Job::new("env", &[])
            .env_clear()
            .env("ONLY_THIS", "yes")
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        // With env cleared, only ONLY_THIS should appear.
        assert!(output.stdout.contains("ONLY_THIS=yes"));
        assert!(!output.stdout.contains("HOME="));
    }

    #[test]
    fn spawn_with_stdin() {
        let handle = Job::new("cat", &[])
            .stdin("hello from stdin")
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        assert_eq!(output.stdout, "hello from stdin");
    }

    #[test]
    fn spawn_nonexistent_command() {
        let result = Job::new("nonexistent_command_12345", &[]).spawn();
        assert!(result.is_err());
    }

    #[test]
    fn spawn_failing_command() {
        let handle = Job::new("sh", &["-c", "exit 42"])
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(!output.success);
        assert_eq!(output.exit_code, Some(42));
    }

    #[test]
    fn spawn_stderr_capture() {
        let handle = Job::new("sh", &["-c", "echo oops >&2"])
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        assert_eq!(output.stderr.trim(), "oops");
    }

    #[test]
    fn job_id_is_nonzero() {
        let handle = Job::new("sleep", &["0"]).spawn().expect("spawn failed");
        assert!(handle.id() > 0);
        let _ = handle.wait();
    }

    #[test]
    fn job_clone_and_spawn_both() {
        let job = Job::new("echo", &["one"]);
        let job2 = job.clone();

        let out1 = job.spawn().unwrap().wait().unwrap();
        let out2 = job2.spawn().unwrap().wait().unwrap();

        assert_eq!(out1.stdout.trim(), "one");
        assert_eq!(out2.stdout.trim(), "one");
    }

    #[test]
    fn spawn_multiline_stdout() {
        let handle = Job::new("sh", &["-c", "echo line1; echo line2; echo line3"])
            .spawn()
            .expect("spawn failed");
        let output = handle.wait().expect("wait failed");
        assert!(output.success);
        let lines: Vec<&str> = output.stdout.lines().collect();
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }
}
