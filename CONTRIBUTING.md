# Contributing

Thanks for helping. ss-belt is small on purpose; the rules below keep it that
way.

## Ground rules

1. **No shell execution.** Every external program is invoked through
   `exec::Cmd` with an argv array. Do not add `sh -c`, and do not interpolate
   user input into a command line.
2. **No hard dependencies on external tools.** A capability that needs a tool
   must implement `support()` and degrade with a clear reason and install hint.
   `ss-belt doctor` on a bare container must still work.
3. **Everything in the registry.** A new capability is a `StepSpec` in
   `adapters::STEPS` plus a module. The CLI, doctor and config validation read
   from that table; do not duplicate knowledge elsewhere.
4. **No platform magic in the core.** Compositor/desktop specifics belong in an
   adapter or in `hotkeys::generate`, not in `pipeline.rs`.

## Adding a step

```rust
// src/adapters/my_tool.rs
pub fn support(probe: &Probe<'_>) -> Support {
    if probe.tools.has(ToolId::MyTool) {
        Support::Ready
    } else {
        Support::Unavailable("my_tool is missing".into())
    }
}

pub fn run(ctx: &mut Ctx) -> Result<()> {
    let input = ctx.require_image("my.step")?;
    if ctx.dry_run {
        ctx.note(format!("dry-run: would process {}", input.display()));
        return Ok(());
    }
    // ... call the tool, set ctx.value
    Ok(())
}
```

Then register it:

```rust
StepSpec {
    name: "my.step",
    summary: "does the thing",
    input: Kind::Image,
    output: Kind::Image,
    support: Some(my_tool::support),
    run: my_tool::run,
}
```

Add a unit test for the adapter and, if it produces a user-visible effect, an
integration test using a fake binary in `tests/fixtures/bin`.

## Testing

```sh
cargo test                  # unit + integration
cargo test --lib            # fast loop
./target/debug/ss-belt doctor --json
```

Integration tests must never touch the real profile: use `SS_BELT_HOME` and a
temporary directory, as the existing tests do.

## Style

- Error messages are user-facing: say what failed, why, and what to do next.
  `user(...).hint(...)` produces the `hint:` line.
- Prefer a small, honest failure over a silent fallback. If a feature cannot
  work on this session type, say so in `support()`.
- Keep `cargo build --release --locked` warning-free.

## Commits

One logical change per commit, imperative subject (`fix(capture): …`,
`feat(cli): …`, `docs: …`). Include the test that proves the change when the
change is behavioural.
