use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::capabilities::{Desktop, ToolId, ToolSet};
use crate::config::Config;
use crate::paths::Paths;
use crate::steps::{self, Probe, Support};

#[derive(Debug, Serialize)]
pub struct ToolRow {
    pub name: String,
    pub found: bool,
    pub path: Option<String>,
    pub purpose: String,
    pub install: String,
}

#[derive(Debug, Serialize)]
pub struct CapabilityRow {
    pub step: String,
    pub summary: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: String,
    pub os: String,
    pub desktop: String,
    pub session: String,
    pub config: String,
    pub save_dir: String,
    pub history: String,
    pub daemon: String,
    pub recording: bool,
    pub capabilities: Vec<CapabilityRow>,
    pub tools: Vec<ToolRow>,
}

pub fn distro() -> (String, String) {
    let text = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut id = String::new();
    let mut pretty = String::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = value.trim_matches('"').to_string();
        }
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            pretty = value.trim_matches('"').to_string();
        }
    }
    if pretty.is_empty() {
        pretty = "Linux".to_string();
    }
    (id, pretty)
}

fn install_command(package: &str) -> String {
    let (id, _) = distro();
    match id.as_str() {
        "fedora" | "rhel" | "centos" => format!("sudo dnf install {package}"),
        "debian" | "ubuntu" | "linuxmint" | "pop" => format!("sudo apt install {package}"),
        "arch" | "manjaro" | "endeavouros" => format!("sudo pacman -S {package}"),
        "opensuse-tumbleweed" | "opensuse-leap" | "suse" => format!("sudo zypper install {package}"),
        _ => format!("install {package} with your package manager"),
    }
}

pub fn collect(paths: &Paths, config: &Config) -> Result<Report> {
    let tools = ToolSet::detect();
    let desktop = Desktop::detect();
    let probe = Probe { tools: &tools, desktop: &desktop, config };

    let capabilities = steps::registry()
        .iter()
        .map(|spec| {
            let support = spec.support(&probe);
            CapabilityRow {
                step: spec.name.to_string(),
                summary: spec.summary.to_string(),
                status: support.label().to_string(),
                detail: support.detail().map(str::to_string),
            }
        })
        .collect();

    let tool_rows = tools
        .iter()
        .map(|(tool, path)| ToolRow {
            name: tool.binary().to_string(),
            found: path.is_some(),
            path: path.map(|p| p.display().to_string()),
            purpose: tool.purpose().to_string(),
            install: install_command(tool.packages().0),
        })
        .collect();

    let daemon = if crate::daemon::is_running(paths) {
        "running".to_string()
    } else {
        "not running".to_string()
    };

    let (_, pretty) = distro();

    Ok(Report {
        version: crate::VERSION.to_string(),
        os: pretty,
        desktop: desktop.label(),
        session: desktop.session.as_str().to_string(),
        config: paths.config_file().display().to_string(),
        save_dir: config.save_dir().display().to_string(),
        history: paths.db_file().display().to_string(),
        daemon,
        recording: crate::adapters::record::is_recording(paths),
        capabilities,
        tools: tool_rows,
    })
}

pub fn print(paths: &Paths, config: &Config, json: bool) -> Result<()> {
    let report = collect(paths, config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("ss-belt {}  ·  {}", report.version, report.os);
    println!("desktop     {}", report.desktop);
    println!("config      {}", report.config);
    println!("save dir    {}{}", report.save_dir, writable_marker(Path::new(&report.save_dir)));
    println!("history     {}", report.history);
    println!("daemon      {}", report.daemon);
    println!("recording   {}", if report.recording { "yes" } else { "no" });
    println!();

    println!("{:<18} {:<12} {}", "CAPABILITY", "STATUS", "DETAIL");
    for row in &report.capabilities {
        let detail = row.detail.clone().unwrap_or_default();
        println!("{:<18} {:<12} {}", row.step, row.status, detail);
    }
    println!();

    let missing: Vec<&ToolRow> = report.tools.iter().filter(|row| !row.found).collect();
    println!("{}/{} optional tools found", report.tools.len() - missing.len(), report.tools.len());
    if !missing.is_empty() {
        println!();
        println!("missing tools:");
        for row in missing {
            println!("  {:<18} {:<38} {}", row.name, row.purpose, row.install);
        }
    }

    Ok(())
}

fn writable_marker(path: &Path) -> &'static str {
    if !path.exists() {
        return "  (will be created)";
    }
    match fs::metadata(path) {
        Ok(meta) if !meta.permissions().readonly() => "",
        Ok(_) => "  (read-only)",
        Err(_) => "  (unreadable)",
    }
}

/// A compact one-line summary used by `ss-belt status`.
pub fn summary_line(paths: &Paths, config: &Config) -> Result<String> {
    let report = collect(paths, config)?;
    let ready = report.capabilities.iter().filter(|row| row.status != "unavailable").count();
    Ok(format!(
        "{} · {}/{} capabilities · daemon {} · {}",
        report.desktop,
        ready,
        report.capabilities.len(),
        report.daemon,
        if report.recording { "recording" } else { "idle" }
    ))
}

pub fn capability_status(support: &Support) -> &'static str {
    support.label()
}

pub fn tool_install(tool: ToolId) -> String {
    install_command(tool.packages().0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distro_is_detected_on_this_host() {
        let (id, pretty) = distro();
        assert!(!pretty.is_empty());
        assert!(!id.contains('\n'));
    }

    #[test]
    fn install_commands_name_a_package_manager() {
        let command = install_command("grim");
        assert!(command.contains("grim"));
        assert!(command.contains("install") || command.contains("-S"));
    }

    #[test]
    fn report_serialises_to_json() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::adapters::testing::paths(dir.path());
        let report = collect(&paths, &Config::default()).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("capabilities"));
        assert!(json.contains("capture.region"));
    }
}
