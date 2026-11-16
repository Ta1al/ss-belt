use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};
use std::path::Path;

use crate::paths::Paths;

#[derive(Debug, Clone)]
pub struct Event<'a> {
    pub run: &'a str,
    pub action: &'a str,
    pub step: &'a str,
    pub ok: bool,
    pub ms: u128,
    pub kind: &'a str,
    pub detail: &'a str,
    pub error: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub id: i64,
    pub ts: String,
    pub run: String,
    pub action: String,
    pub step: String,
    pub ok: bool,
    pub ms: i64,
    pub kind: String,
    pub detail: String,
    pub error: Option<String>,
}

impl Row {
    pub fn line(&self) -> String {
        let status = if self.ok { "ok  " } else { "FAIL" };
        let mut out = format!(
            "{} {} {:<16} {:<22} {:>6}ms {}",
            self.ts, status, self.action, self.step, self.ms, self.detail
        );
        if let Some(error) = &self.error {
            out.push_str("  ! ");
            out.push_str(error);
        }
        out
    }
}

pub struct History {
    conn: Connection,
}

impl History {
    pub fn open(paths: &Paths) -> Result<Self> {
        if let Some(parent) = paths.db_file().parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::open_path(&paths.db_file())
    }

    pub fn open_path(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening history database {}", path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS events (
                 id     INTEGER PRIMARY KEY AUTOINCREMENT,
                 ts     TEXT NOT NULL,
                 run    TEXT NOT NULL,
                 action TEXT NOT NULL,
                 step   TEXT NOT NULL,
                 ok     INTEGER NOT NULL,
                 ms     INTEGER NOT NULL,
                 kind   TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 error  TEXT
             );
             CREATE INDEX IF NOT EXISTS events_ts ON events(ts DESC);",
        )?;
        Ok(Self { conn })
    }

    pub fn record(&self, event: &Event<'_>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (ts, run, action, step, ok, ms, kind, detail, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                Local::now().to_rfc3339(),
                event.run,
                event.action,
                event.step,
                event.ok as i64,
                event.ms as i64,
                event.kind,
                event.detail,
                event.error
            ],
        )?;
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<Row>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts, run, action, step, ok, ms, kind, detail, error
             FROM events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok(Row {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    run: row.get(2)?,
                    action: row.get(3)?,
                    step: row.get(4)?,
                    ok: row.get::<_, i64>(5)? == 1,
                    ms: row.get(6)?,
                    kind: row.get(7)?,
                    detail: row.get(8)?,
                    error: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Keep at most `limit` events, and drop anything older than `days` when
    /// `days` is non-zero. Returns the number of deleted rows.
    pub fn prune(&self, limit: usize, days: u32) -> Result<usize> {
        let mut removed = self.conn.execute(
            "DELETE FROM events WHERE id <= (SELECT MAX(id) FROM events) - ?1",
            [limit as i64],
        )?;
        if days > 0 {
            let cutoff = Local::now() - chrono::Duration::days(days as i64);
            removed += self.conn.execute(
                "DELETE FROM events WHERE ts < ?1",
                [cutoff.to_rfc3339()],
            )?;
        }
        Ok(removed)
    }

    pub fn count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn uploads(&self, limit: usize) -> Result<Vec<Row>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts, run, action, step, ok, ms, kind, detail, error
             FROM events WHERE step = 'upload' ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok(Row {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    run: row.get(2)?,
                    action: row.get(3)?,
                    step: row.get(4)?,
                    ok: row.get::<_, i64>(5)? == 1,
                    ms: row.get(6)?,
                    kind: row.get(7)?,
                    detail: row.get(8)?,
                    error: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_history() -> (tempfile::TempDir, History) {
        let dir = tempfile::tempdir().unwrap();
        let history = History::open_path(&dir.path().join("h.sqlite")).unwrap();
        (dir, history)
    }

    #[test]
    fn events_round_trip() {
        let (_dir, history) = temp_history();
        history
            .record(&Event {
                run: "r1",
                action: "region",
                step: "capture.region",
                ok: true,
                ms: 42,
                kind: "image",
                detail: "/tmp/a.png",
                error: None,
            })
            .unwrap();
        let rows = history.recent(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].step, "capture.region");
        assert!(rows[0].ok);
        assert_eq!(history.count().unwrap(), 1);
    }

    #[test]
    fn pruning_keeps_the_newest_events() {
        let (_dir, history) = temp_history();
        for i in 0..10 {
            history
                .record(&Event {
                    run: "r",
                    action: "a",
                    step: &format!("s{i}"),
                    ok: true,
                    ms: 1,
                    kind: "text",
                    detail: "d",
                    error: None,
                })
                .unwrap();
        }
        history.prune(3, 0).unwrap();
        let rows = history.recent(100).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].step, "s9");
    }
}
