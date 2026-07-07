use crate::models::{ChangeRecord, PapersSession};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("Could not open Papers state: {error}"))?;
        connection
            .execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                PRAGMA foreign_keys = ON;

                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    hermes_session_id TEXT,
                    title TEXT NOT NULL,
                    mode TEXT NOT NULL CHECK(mode IN ('operator', 'builder')),
                    state TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    event_type TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(session_id) REFERENCES sessions(id)
                );

                CREATE TABLE IF NOT EXISTS approvals (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    action_kind TEXT NOT NULL,
                    target TEXT NOT NULL,
                    preview_json TEXT NOT NULL,
                    decision TEXT,
                    created_at TEXT NOT NULL,
                    decided_at TEXT
                );

                CREATE TABLE IF NOT EXISTS changes (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    request TEXT NOT NULL,
                    selection_json TEXT,
                    status TEXT NOT NULL,
                    branch TEXT NOT NULL,
                    worktree_path TEXT NOT NULL,
                    base_commit TEXT NOT NULL,
                    accepted_commit TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS sync_queue (
                    id TEXT PRIMARY KEY,
                    commit_id TEXT NOT NULL,
                    remote TEXT NOT NULL,
                    state TEXT NOT NULL,
                    last_error TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                "#,
            )
            .map_err(|error| format!("Could not initialize Papers state: {error}"))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<PapersSession>, String> {
        let connection = self.connection.lock().map_err(|_| "State lock failed")?;
        let mut statement = connection
            .prepare(
                "SELECT id, hermes_session_id, title, mode, state, created_at, updated_at
                 FROM sessions ORDER BY updated_at DESC LIMIT 100",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(PapersSession {
                    id: row.get(0)?,
                    hermes_session_id: row.get(1)?,
                    title: row.get(2)?,
                    mode: row.get(3)?,
                    state: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn create_session(&self, title: &str, mode: &str) -> Result<PapersSession, String> {
        if !matches!(mode, "operator" | "builder") {
            return Err("Unknown agent mode".to_string());
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = if title.trim().is_empty() {
            "New conversation"
        } else {
            title.trim()
        };
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "INSERT INTO sessions (id, title, mode, state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'idle', ?4, ?4)",
                params![id, title, mode, now],
            )
            .map_err(|error| error.to_string())?;
        Ok(PapersSession {
            id,
            hermes_session_id: None,
            title: title.to_string(),
            mode: mode.to_string(),
            state: "idle".to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn rename_session(&self, id: &str, title: &str) -> Result<PapersSession, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Conversation title cannot be empty".to_string());
        }
        if title.chars().count() > 140 {
            return Err("Conversation title is too long".to_string());
        }
        let changed = self
            .connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, title, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("Conversation no longer exists".to_string());
        }
        self.session(id)
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let mut connection = self.connection.lock().map_err(|_| "State lock failed")?;
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM events WHERE session_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM approvals WHERE session_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        let changed = transaction
            .execute("DELETE FROM sessions WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("Conversation no longer exists".to_string());
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn session(&self, id: &str) -> Result<PapersSession, String> {
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .query_row(
                "SELECT id, hermes_session_id, title, mode, state, created_at, updated_at
                 FROM sessions WHERE id = ?1",
                [id],
                |row| {
                    Ok(PapersSession {
                        id: row.get(0)?,
                        hermes_session_id: row.get(1)?,
                        title: row.get(2)?,
                        mode: row.get(3)?,
                        state: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(|_| "Conversation no longer exists".to_string())
    }

    pub fn bind_hermes_session(&self, id: &str, hermes_id: &str) -> Result<(), String> {
        let changed = self
            .connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "UPDATE sessions SET hermes_session_id = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, hermes_id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("Conversation no longer exists".to_string());
        }
        Ok(())
    }

    pub fn update_session_state(&self, id: &str, state: &str) -> Result<(), String> {
        const STATES: &[&str] = &[
            "idle",
            "planning",
            "acting",
            "awaiting_approval",
            "previewing",
            "completed",
            "paused",
            "cancelled",
            "failed",
        ];
        if !STATES.contains(&state) {
            return Err("Invalid run state".to_string());
        }
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "UPDATE sessions SET state = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, state, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn record_event(&self, session_id: &str, event: &Value) -> Result<(), String> {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let Some(payload) = sanitized_event_payload(event_type, event) else {
            return Ok(());
        };
        let connection = self.connection.lock().map_err(|_| "State lock failed")?;
        let sequence: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM events WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO events (session_id, sequence, event_type, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session_id,
                    sequence,
                    event_type,
                    payload.to_string(),
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn insert_change(
        &self,
        record: &ChangeRecord,
        selection: Option<&Value>,
    ) -> Result<(), String> {
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "INSERT INTO changes
                 (id, title, request, selection_json, status, branch, worktree_path,
                  base_commit, accepted_commit, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    record.id,
                    record.title,
                    record.request,
                    selection.map(Value::to_string),
                    record.status,
                    record.branch,
                    record.worktree_path,
                    record.base_commit,
                    record.accepted_commit,
                    record.created_at,
                    record.updated_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn list_changes(&self) -> Result<Vec<ChangeRecord>, String> {
        let connection = self.connection.lock().map_err(|_| "State lock failed")?;
        let mut statement = connection
            .prepare(
                "SELECT id, title, request, status, branch, worktree_path, base_commit,
                        accepted_commit, created_at, updated_at
                 FROM changes ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(ChangeRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    request: row.get(2)?,
                    status: row.get(3)?,
                    branch: row.get(4)?,
                    worktree_path: row.get(5)?,
                    base_commit: row.get(6)?,
                    accepted_commit: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn change(&self, id: &str) -> Result<ChangeRecord, String> {
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .query_row(
                "SELECT id, title, request, status, branch, worktree_path, base_commit,
                        accepted_commit, created_at, updated_at FROM changes WHERE id = ?1",
                [id],
                |row| {
                    Ok(ChangeRecord {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        request: row.get(2)?,
                        status: row.get(3)?,
                        branch: row.get(4)?,
                        worktree_path: row.get(5)?,
                        base_commit: row.get(6)?,
                        accepted_commit: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .map_err(|_| "Self-edit record was not found".to_string())
    }

    pub fn update_change(
        &self,
        id: &str,
        status: &str,
        accepted_commit: Option<&str>,
    ) -> Result<ChangeRecord, String> {
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "UPDATE changes
                 SET status = ?2, accepted_commit = COALESCE(?3, accepted_commit), updated_at = ?4
                 WHERE id = ?1",
                params![id, status, accepted_commit, Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        self.change(id)
    }

    pub fn enqueue_sync(&self, commit: &str, error: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.connection
            .lock()
            .map_err(|_| "State lock failed")?
            .execute(
                "INSERT INTO sync_queue
                 (id, commit_id, remote, state, last_error, created_at, updated_at)
                 VALUES (?1, ?2, 'origin/main', 'pending', ?3, ?4, ?4)",
                params![Uuid::new_v4().to_string(), commit, error, now],
            )
            .map_err(|db_error| db_error.to_string())?;
        Ok(())
    }
}

fn sanitized_event_payload(event_type: &str, event: &Value) -> Option<Value> {
    if is_private_reasoning_event(event_type) {
        return None;
    }

    let mut payload = event.get("payload").cloned().unwrap_or(Value::Null);
    redact_private_reasoning_fields(&mut payload);
    Some(payload)
}

fn is_private_reasoning_event(event_type: &str) -> bool {
    let normalized = event_type.to_ascii_lowercase();
    normalized.starts_with("reasoning.")
        || normalized.starts_with("thinking.")
        || normalized.starts_with("chain_of_thought.")
}

fn redact_private_reasoning_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for key in [
                "reasoning",
                "thinking",
                "chain_of_thought",
                "chainOfThought",
                "cot",
            ] {
                map.remove(key);
            }
            for child in map.values_mut() {
                redact_private_reasoning_fields(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                redact_private_reasoning_fields(child);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_updates_sessions() {
        let db = Database {
            connection: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
        };
        db.connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TABLE sessions (
                    id TEXT PRIMARY KEY, hermes_session_id TEXT, title TEXT, mode TEXT,
                    state TEXT, created_at TEXT, updated_at TEXT
                );
                CREATE TABLE events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    event_type TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE approvals (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    action_kind TEXT NOT NULL,
                    target TEXT NOT NULL,
                    preview_json TEXT NOT NULL,
                    decision TEXT,
                    created_at TEXT NOT NULL,
                    decided_at TEXT
                );",
            )
            .unwrap();
        let session = db.create_session("A useful task", "operator").unwrap();
        db.update_session_state(&session.id, "acting").unwrap();
        assert_eq!(db.list_sessions().unwrap()[0].state, "acting");
        assert_eq!(
            db.rename_session(&session.id, "Better title").unwrap().title,
            "Better title"
        );
        db.delete_session(&session.id).unwrap();
        assert!(db.list_sessions().unwrap().is_empty());
    }

    #[test]
    fn does_not_store_private_reasoning_events_or_fields() {
        let db = Database {
            connection: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
        };
        db.connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TABLE events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    event_type TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );",
            )
            .unwrap();

        db.record_event(
            "session-1",
            &serde_json::json!({
                "type": "reasoning.delta",
                "payload": { "text": "private thinking" }
            }),
        )
        .unwrap();
        db.record_event(
            "session-1",
            &serde_json::json!({
                "type": "message.complete",
                "payload": {
                    "text": "public answer",
                    "reasoning": "private thinking",
                    "nested": { "chain_of_thought": "also private" }
                }
            }),
        )
        .unwrap();

        let connection = db.connection.lock().unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let payload: String = connection
            .query_row("SELECT payload_json FROM events", [], |row| row.get(0))
            .unwrap();
        assert!(payload.contains("public answer"));
        assert!(!payload.contains("private thinking"));
        assert!(!payload.contains("chain_of_thought"));
    }
}
