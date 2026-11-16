use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Every file ss-belt reads or writes.
///
/// `SS_BELT_HOME` relocates all of them under one directory, which is what the
/// integration tests use so a test run can never touch a real profile.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: Option<PathBuf>,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Self {
        if let Some(root) = env::var_os("SS_BELT_HOME").map(PathBuf::from) {
            return Self {
                config_dir: root.join("config"),
                data_dir: root.join("data"),
                cache_dir: root.join("cache"),
                runtime_dir: root.join("run"),
                root: Some(root),
            };
        }

        let (config_dir, data_dir, cache_dir) = match directories::ProjectDirs::from("", "", "ss-belt")
        {
            Some(dirs) => (
                dirs.config_dir().to_path_buf(),
                dirs.data_dir().to_path_buf(),
                dirs.cache_dir().to_path_buf(),
            ),
            None => (
                PathBuf::from(".config/ss-belt"),
                PathBuf::from(".local/share/ss-belt"),
                PathBuf::from(".cache/ss-belt"),
            ),
        };

        let runtime_dir = match env::var_os("XDG_RUNTIME_DIR") {
            Some(dir) => PathBuf::from(dir).join("ss-belt"),
            None => data_dir.join("run"),
        };

        Self { root: None, config_dir, data_dir, cache_dir, runtime_dir }
    }

    pub fn ensure(&self) -> io::Result<()> {
        for dir in [&self.config_dir, &self.data_dir, &self.cache_dir, &self.runtime_dir] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn uploaders_dir(&self) -> PathBuf {
        self.config_dir.join("uploaders")
    }

    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("history.sqlite")
    }

    pub fn socket_file(&self) -> PathBuf {
        self.runtime_dir.join(crate::SOCKET_NAME)
    }

    pub fn lock_file(&self) -> PathBuf {
        self.runtime_dir.join("daemon.lock")
    }

    pub fn state_file(&self) -> PathBuf {
        self.runtime_dir.join("state.json")
    }

    pub fn last_file(&self) -> PathBuf {
        self.data_dir.join("last")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("daemon.log")
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.cache_dir.join("staging")
    }

    /// A fresh directory for one pipeline run. Removed by the runner on success
    /// unless a step asks to keep it.
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.staging_dir().join(run_id)
    }

    pub fn home(&self) -> PathBuf {
        env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
    }
}

pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
