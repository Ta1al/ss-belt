use anyhow::Result;
use std::time::Instant;

use crate::action::{DEFAULT_ACTIONS, describe_action};
use crate::config::Config;
use crate::error::{is_cancelled, user};
use crate::history::{Event, History};
use crate::steps::{self, Ctx};
use crate::error::HintExt;

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<String>,
    pub custom: bool,
    /// The kind already in hand when the pipeline starts.
    pub start: crate::steps::Kind,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub action: String,
    pub steps_run: usize,
    pub ms: u128,
    pub value: String,
    pub cancelled: bool,
}

impl Pipeline {
    pub fn resolve(config: &Config, action: &str) -> Result<Self> {
        if let Some(custom) = config.actions.get(action) {
            return Ok(Self {
                name: action.to_string(),
                steps: custom.steps.clone(),
                custom: true,
                start: crate::steps::Kind::None,
            });
        }
        let steps = DEFAULT_ACTIONS
            .iter()
            .find(|(name, _)| *name == action)
            .map(|(_, steps)| steps.iter().map(|s| (*s).to_string()).collect())
            .ok_or_else(|| {
                user(format!("unknown action `{action}`")).hint("run `ss-belt actions` for the list")
            })?;
        Ok(Self { name: action.to_string(), steps, custom: false, start: crate::steps::Kind::None })
    }

    pub fn validate(&self) -> Result<()> {
        steps::validate_chain_from(&self.steps, self.start)?;
        Ok(())
    }

    pub fn describe(&self) -> String {
        let mut out = format!("{}: {}", self.name, describe_action(&self.name));
        out.push_str("\n  ");
        out.push_str(&self.steps.join(" -> "));
        out
    }
}

pub fn run(ctx: &mut Ctx, pipeline: &Pipeline) -> Result<Report> {
    pipeline.validate()?;

    let history = if ctx.config.general.history {
        History::open(&ctx.paths).ok()
    } else {
        None
    };
    let run_id = ctx
        .run_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "run".to_string());

    let started = Instant::now();
    let mut steps_run = 0;

    for name in &pipeline.steps {
        let spec = steps::find(name)
            .ok_or_else(|| user(format!("unknown step `{name}`")))?;

        if !spec.input.accepts(ctx.value.kind()) {
            return Err(user(format!(
                "step `{name}` needs {} but the pipeline has {} at that point",
                spec.input.as_str(),
                ctx.value.kind().as_str()
            )));
        }

        let step_started = Instant::now();
        ctx.note(format!("-> {} ({})", spec.name, spec.summary));

        // Steps receive the dry-run flag themselves so each one can describe
        // exactly what it would have done (and set a plausible value).
        let result = (spec.run)(ctx);

        let ms = step_started.elapsed().as_millis();
        match result {
            Ok(()) => {
                steps_run += 1;
                if let Some(history) = &history {
                    let detail = ctx.value.describe();
                    let _ = history.record(&Event {
                        run: &run_id,
                        action: &pipeline.name,
                        step: spec.name,
                        ok: true,
                        ms,
                        kind: ctx.value.kind().as_str(),
                        detail: &detail,
                        error: None,
                    });
                }
            }
            Err(err) if is_cancelled(&err) => {
                if let Some(history) = &history {
                    let _ = history.record(&Event {
                        run: &run_id,
                        action: &pipeline.name,
                        step: spec.name,
                        ok: false,
                        ms,
                        kind: ctx.value.kind().as_str(),
                        detail: "cancelled by user",
                        error: None,
                    });
                }
                prune(&history, &ctx.config);
                return Ok(Report {
                    action: pipeline.name.clone(),
                    steps_run,
                    ms: started.elapsed().as_millis(),
                    value: "cancelled".to_string(),
                    cancelled: true,
                });
            }
            Err(err) => {
                let message = format!("{err:#}");
                if let Some(history) = &history {
                    let _ = history.record(&Event {
                        run: &run_id,
                        action: &pipeline.name,
                        step: spec.name,
                        ok: false,
                        ms,
                        kind: ctx.value.kind().as_str(),
                        detail: "failed",
                        error: Some(&message),
                    });
                }
                prune(&history, &ctx.config);
                return Err(err);
            }
        }
    }

    prune(&history, &ctx.config);

    Ok(Report {
        action: pipeline.name.clone(),
        steps_run,
        ms: started.elapsed().as_millis(),
        value: ctx.value.describe(),
        cancelled: false,
    })
}

fn prune(history: &Option<History>, config: &Config) {
    if let Some(history) = history {
        let _ = history.prune(config.general.history_limit, config.general.retain_days);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{Desktop, SessionType, ToolSet};
    use crate::config::Config;
    use crate::paths::Paths;

    fn temp_ctx(action: &str, dry_run: bool) -> Ctx {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::mem::forget(dir);
        let paths = Paths {
            root: Some(root.clone()),
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            runtime_dir: root.join("run"),
        };
        paths.ensure().unwrap();
        let mut config = Config::default();
        config.general.save_dir = root.join("shots").display().to_string();
        let mut ctx = Ctx::new(
            paths,
            config,
            ToolSet::default(),
            Desktop {
                session: SessionType::X11,
                desktop: "test".into(),
                is_wlroots: false,
                has_xwayland: true,
            },
            action,
        )
        .unwrap();
        ctx.dry_run = dry_run;
        ctx.tools = crate::adapters::testing::tools(&[
            crate::capabilities::ToolId::Grim,
            crate::capabilities::ToolId::Slurp,
            crate::capabilities::ToolId::WlCopy,
            crate::capabilities::ToolId::WlPaste,
        ]);
        ctx.desktop = crate::adapters::testing::wayland();
        ctx
    }

    #[test]
    fn every_default_action_is_a_valid_chain() {
        for (name, steps) in DEFAULT_ACTIONS {
            let chain: Vec<String> = steps.iter().map(|s| (*s).to_string()).collect();
            steps::validate_chain(&chain).unwrap_or_else(|err| panic!("{name}: {err}"));
        }
    }

    #[test]
    fn dry_run_executes_the_whole_chain_without_tools() {
        let mut ctx = temp_ctx("region", true);
        let pipeline = Pipeline::resolve(&ctx.config, "region").unwrap();
        let report = run(&mut ctx, &pipeline).unwrap();
        assert!(!report.cancelled);
        assert_eq!(report.steps_run, pipeline.steps.len());
    }

    #[test]
    fn custom_action_overrides_the_default() {
        let mut ctx = temp_ctx("region", true);
        ctx.config.actions.insert(
            "region".into(),
            crate::config::ActionConfig { steps: vec!["notify".into()] },
        );
        let pipeline = Pipeline::resolve(&ctx.config, "region").unwrap();
        assert!(pipeline.custom);
        assert_eq!(pipeline.steps, vec!["notify".to_string()]);
    }

    #[test]
    fn unknown_action_is_a_user_error() {
        let ctx = temp_ctx("region", true);
        let err = Pipeline::resolve(&ctx.config, "nope").unwrap_err();
        assert!(crate::error::is_user_error(&err));
    }

    #[test]
    fn broken_chain_is_rejected_before_running() {
        let chain = vec!["annotate".to_string()];
        let err = steps::validate_chain(&chain).unwrap_err();
        assert!(format!("{err}").contains("needs image"));
    }
}
