use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{user, user_hint};

/// A single external program invocation.
///
/// Arguments are always passed as separate argv entries: ss-belt never builds a
/// shell command line, so a filename containing `;` or `$(...)` is just a filename.
#[derive(Debug, Clone)]
pub struct Cmd {
    program: String,
    args: Vec<String>,
    stdin: Option<Vec<u8>>,
    envs: Vec<(String, String)>,
    cwd: Option<PathBuf>,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

impl Output {
    pub fn success(&self) -> bool {
        !self.timed_out && self.code == Some(0)
    }

    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }

    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }

    /// Best available diagnostic for a failed run.
    pub fn failure(&self) -> String {
        if self.timed_out {
            return "timed out".to_string();
        }
        let stderr = self.stderr_str();
        if !stderr.is_empty() {
            stderr
        } else {
            format!("exited with {:?}", self.code)
        }
    }
}

impl Cmd {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            stdin: None,
            envs: Vec::new(),
            cwd: None,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn arg_path(self, path: &std::path::Path) -> Self {
        self.arg(path.display().to_string())
    }

    pub fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(bytes.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    pub fn cwd(mut self, dir: &std::path::Path) -> Self {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for arg in &self.args {
            if arg.is_empty() || arg.contains(char::is_whitespace) {
                out.push_str(&format!(" '{}'", arg.replace('\'', r"'\''")));
            } else {
                out.push(' ');
                out.push_str(arg);
            }
        }
        out
    }

    pub fn run(&self) -> Result<Output> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(if self.stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &self.envs {
            command.env(key, value);
        }
        if let Some(dir) = &self.cwd {
            command.current_dir(dir);
        }

        let mut child = command.spawn().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                user_hint(
                    format!("`{}` is not installed", self.program),
                    "run `ss-belt doctor` to see which optional tools are missing",
                )
            } else {
                anyhow::Error::new(err).context(format!("could not run `{}`", self.program))
            }
        })?;

        let stdin_handle = self.stdin.clone().map(|bytes| {
            let mut pipe = child.stdin.take().expect("stdin piped");
            std::thread::spawn(move || {
                let _ = pipe.write_all(&bytes);
            })
        });

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");
        let stdout_handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf);
            buf
        });
        let stderr_handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf);
            buf
        });

        let deadline = Instant::now() + self.timeout;
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        timed_out = true;
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(err) => return Err(err).context(format!("waiting for `{}`", self.program)),
            }
        };

        if let Some(handle) = stdin_handle {
            let _ = handle.join();
        }
        let stdout = stdout_handle.join().unwrap_or_default();
        let stderr = stderr_handle.join().unwrap_or_default();

        Ok(Output {
            code: status.and_then(|s| s.code()),
            stdout,
            stderr,
            timed_out,
        })
    }

    /// Run and require exit status 0, returning stdout/stderr on success.
    pub fn checked(&self) -> Result<Output> {
        let output = self.run()?;
        if output.success() {
            Ok(output)
        } else {
            Err(user(format!("`{}` failed: {}", self.display(), output.failure())))
        }
    }
}

/// PATH lookup without a shell.
pub fn which(program: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let path = PathBuf::from(program);
        return crate::paths::is_executable(&path).then_some(path);
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if crate::paths::is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_are_never_shell_expanded() {
        let output = Cmd::new("printf")
            .arg("%s")
            .arg("$(id); `id`; $HOME")
            .checked()
            .unwrap();
        assert_eq!(output.stdout_str(), "$(id); `id`; $HOME");
    }

    #[test]
    fn missing_program_is_a_user_error_with_hint() {
        let err = Cmd::new("ss-belt-definitely-not-installed").run().unwrap_err();
        assert!(crate::error::is_user_error(&err));
        assert!(format!("{err}").contains("not installed"));
    }

    #[test]
    fn nonzero_exit_reports_stderr() {
        let err = Cmd::new("sh").args(["-c", "echo bad >&2; exit 3"]).checked().unwrap_err();
        assert!(format!("{err}").contains("bad"));
    }

    #[test]
    fn stdin_is_delivered() {
        let out = Cmd::new("cat").stdin("hello belt").checked().unwrap();
        assert_eq!(out.stdout_str(), "hello belt");
    }

    #[test]
    fn timeouts_kill_the_child() {
        let out = Cmd::new("sleep")
            .arg("5")
            .timeout(Duration::from_millis(120))
            .run()
            .unwrap();
        assert!(out.timed_out);
        assert!(!out.success());
    }
}
