use crate::protocol::{Action, Event};
use rusqlite::{Connection, params};
use serde_json::Value;
pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let s = Self {
            connection: Connection::open(path)?,
        };
        s.migrate()?;
        Ok(s)
    }
    pub fn migrate(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY); CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT,timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),agent TEXT NOT NULL,project_dir TEXT,event_type TEXT NOT NULL,tool_name TEXT,tool_input TEXT NOT NULL,decision TEXT NOT NULL,policy_id INTEGER,pid INTEGER,session_id TEXT,metadata TEXT); CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp); CREATE TABLE IF NOT EXISTS policies(id INTEGER PRIMARY KEY AUTOINCREMENT,scope TEXT NOT NULL,agent TEXT,project_dir TEXT,tool_name TEXT,pattern TEXT,action TEXT NOT NULL,priority INTEGER NOT NULL DEFAULT 0,enabled INTEGER NOT NULL DEFAULT 1); CREATE UNIQUE INDEX IF NOT EXISTS idx_policy_unique ON policies(scope,agent,project_dir,tool_name,pattern); INSERT OR IGNORE INTO schema_migrations(version) VALUES(1);")?;
        Ok(())
    }
    pub fn persist(
        &self,
        e: &Event,
        action: Action,
        policy_id: Option<i64>,
        input: &Value,
    ) -> rusqlite::Result<i64> {
        self.connection.execute("INSERT INTO events(agent,project_dir,event_type,tool_name,tool_input,decision,policy_id,pid,session_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![e.agent,e.project_dir,e.event_type,e.tool_name,input.to_string(),format!("{:?}",action).to_lowercase(),policy_id,e.pid,e.session_id])?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn recent_events(&self, limit: usize) -> rusqlite::Result<Vec<Value>> {
        let mut statement = self.connection.prepare("SELECT id, timestamp, agent, event_type, tool_name, decision FROM events ORDER BY id DESC LIMIT ?1")?;
        statement.query_map([limit as i64], |row| Ok(serde_json::json!({"id":row.get::<_, i64>(0)?,"timestamp":row.get::<_, String>(1)?,"agent":row.get::<_, String>(2)?,"event_type":row.get::<_, String>(3)?,"tool_name":row.get::<_, Option<String>>(4)?,"decision":row.get::<_, String>(5)?})))?.collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protocol::CapabilityState, redaction::redact};
    use serde_json::json;
    #[test]
    fn migrations_and_redacted_event_work() {
        let db = tempfile::NamedTempFile::new().unwrap();
        let store = Store::open(db.path().to_str().unwrap()).unwrap();
        let e = Event {
            agent: "a".into(),
            project_dir: None,
            event_type: "tool".into(),
            tool_name: None,
            tool_input: json!({"token":"secret"}),
            pid: None,
            session_id: None,
            capability: CapabilityState::Enforced,
        };
        store
            .persist(&e, Action::Deny, None, &redact(&e.tool_input))
            .unwrap();
        let raw: String = store
            .connection
            .query_row("SELECT tool_input FROM events", [], |r| r.get(0))
            .unwrap();
        assert!(!raw.contains("secret"));
        assert!(raw.contains("REDACTED"));
    }
}
