use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct DbManager {
    conn: Arc<Mutex<Connection>>,
}

impl DbManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        manager.init_schema()?;
        Ok(manager)
    }

    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS connection_config (
                id INTEGER PRIMARY KEY,
                connection_type TEXT NOT NULL,
                options_json TEXT NOT NULL,
                is_connected INTEGER NOT NULL DEFAULT 0
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS dpt_configs (
                group_address TEXT PRIMARY KEY,
                dpt TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                group_address TEXT PRIMARY KEY
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS config_retention (
                key TEXT PRIMARY KEY,
                value_seconds INTEGER NOT NULL
            );",
            [],
        )?;

        // Initialize default retention (e.g. 7 days = 604800 seconds)
        conn.execute(
            "INSERT OR IGNORE INTO config_retention (key, value_seconds) VALUES ('retention', 604800);",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS indications_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                group_address TEXT NOT NULL,
                cemi_raw BLOB NOT NULL,
                description TEXT NOT NULL,
                value TEXT
            );",
            [],
        )?;

        Ok(())
    }

    pub fn save_connection_config(&self, conn_type: &str, opts_json: &str, is_connected: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO connection_config (id, connection_type, options_json, is_connected)
             VALUES (1, ?1, ?2, ?3);",
            params![conn_type, opts_json, if is_connected { 1 } else { 0 }],
        )?;
        Ok(())
    }

    pub fn get_connection_config(&self) -> Result<Option<(String, String, bool)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT connection_type, options_json, is_connected FROM connection_config WHERE id = 1;")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let conn_type: String = row.get(0)?;
            let opts_json: String = row.get(1)?;
            let is_connected_val: i32 = row.get(2)?;
            Ok(Some((conn_type, opts_json, is_connected_val == 1)))
        } else {
            Ok(None)
        }
    }

    pub fn save_dpt_config(&self, addr: &str, dpt: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO dpt_configs (group_address, dpt) VALUES (?1, ?2);",
            params![addr, dpt],
        )?;
        Ok(())
    }

    pub fn remove_dpt_config(&self, addr: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM dpt_configs WHERE group_address = ?1;", params![addr])?;
        Ok(())
    }

    pub fn get_dpt_configs(&self) -> Result<Vec<(String, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT group_address, dpt FROM dpt_configs;")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn add_subscription(&self, addr: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR IGNORE INTO subscriptions (group_address) VALUES (?1);", params![addr])?;
        Ok(())
    }

    pub fn remove_subscription(&self, addr: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM subscriptions WHERE group_address = ?1;", params![addr])?;
        Ok(())
    }

    pub fn get_subscriptions(&self) -> Result<Vec<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT group_address FROM subscriptions;")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn set_retention(&self, seconds: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR REPLACE INTO config_retention (key, value_seconds) VALUES ('retention', ?1);", params![seconds])?;
        Ok(())
    }

    pub fn get_retention(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value_seconds FROM config_retention WHERE key = 'retention';")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(604800) // Default to 7 days
        }
    }

    pub fn save_indication(&self, timestamp: i64, addr: &str, cemi_raw: &[u8], description: &str, value: Option<&str>) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO indications_history (timestamp, group_address, cemi_raw, description, value)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![timestamp, addr, cemi_raw, description, value],
        )?;
        Ok(())
    }

    pub fn clean_old_indications(&self, retention_seconds: i64) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let limit = now - retention_seconds;
        let deleted = conn.execute("DELETE FROM indications_history WHERE timestamp < ?1;", params![limit])?;
        Ok(deleted)
    }

    pub fn get_indications_history(&self, limit: usize) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, group_address, cemi_raw, description, value
             FROM indications_history ORDER BY id DESC LIMIT ?1;"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let id: i64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let group_address: String = row.get(2)?;
            let cemi_raw: Vec<u8> = row.get(3)?;
            let description: String = row.get(4)?;
            let value: Option<String> = row.get(5)?;
            
            let cemi_hex = hex_encode(cemi_raw);

            Ok(serde_json::json!({
                "id": id,
                "timestamp": timestamp,
                "group_address": group_address,
                "cemi_hex": cemi_hex,
                "description": description,
                "value": value
            }))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}

fn hex_encode(data: Vec<u8>) -> String {
    let mut s = String::new();
    for byte in data {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}
