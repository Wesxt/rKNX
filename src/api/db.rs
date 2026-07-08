use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct DbManager {
    conn: Arc<Mutex<Connection>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupAddressSubscription {
    pub address: String,
    pub dpt: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub last_value: serde_json::Value,
    pub updated_at: Option<String>,
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

        // 1. Connection Config Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS connection_config (
                id INTEGER PRIMARY KEY,
                connection_type TEXT NOT NULL,
                options_json TEXT NOT NULL,
                is_connected INTEGER NOT NULL DEFAULT 0
            );",
            [],
        )?;

        // 2. Group Addresses Table (matches personal-home-core)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS group_addresses (
                address TEXT PRIMARY KEY,
                dpt TEXT,
                last_value TEXT,
                name TEXT,
                description TEXT,
                updated_at TEXT
            );",
            [],
        )?;

        // 3. Retention Config Table
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

        // 4. Indications History Table
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

    pub fn save_connection_config(
        &self,
        conn_type: &str,
        opts_json: &str,
        is_connected: bool,
    ) -> Result<(), rusqlite::Error> {
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
        let mut stmt = conn.prepare(
            "SELECT connection_type, options_json, is_connected FROM connection_config WHERE id = 1;"
        )?;
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

    // Upsert a group address (subscription) into SQLite
    pub fn save_subscription(
        &self,
        address: &str,
        dpt: Option<&str>,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO group_addresses (address, dpt, name, description, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(address) DO UPDATE SET
                dpt = COALESCE(?5, dpt),
                name = COALESCE(?6, name),
                description = COALESCE(?7, description),
                updated_at = datetime('now');",
            params![address, dpt, name, description, dpt, name, description],
        )?;
        Ok(())
    }

    // Delete subscription from SQLite
    pub fn delete_subscription(&self, address: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM group_addresses WHERE address = ?1;",
            params![address],
        )?;
        Ok(())
    }

    // Update the last decoded value of a group address
    pub fn update_last_value(&self, address: &str, val_json: Option<&str>) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE group_addresses SET last_value = ?1, updated_at = datetime('now') WHERE address = ?2;",
            params![val_json, address],
        )?;
        Ok(())
    }

    // Get all subscriptions from SQLite
    pub fn get_all_subscriptions(&self) -> Result<Vec<GroupAddressSubscription>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT address, dpt, last_value, name, description, updated_at FROM group_addresses;"
        )?;
        let rows = stmt.query_map([], |row| {
            let address: String = row.get(0)?;
            let dpt: Option<String> = row.get(1)?;
            let last_value_str: Option<String> = row.get(2)?;
            let name: Option<String> = row.get(3)?;
            let description: Option<String> = row.get(4)?;
            let updated_at: Option<String> = row.get(5)?;

            let last_value = last_value_str
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or(serde_json::Value::Null);

            Ok(GroupAddressSubscription {
                address,
                dpt,
                name,
                description,
                last_value,
                updated_at,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn save_indication(
        &self,
        timestamp: i64,
        group_address: &str,
        cemi_raw: &[u8],
        description: &str,
        value: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO indications_history (timestamp, group_address, cemi_raw, description, value)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![timestamp, group_address, cemi_raw, description, value],
        )?;
        Ok(())
    }

    pub fn clean_old_indications(&self, retention_seconds: i64) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let limit = now - retention_seconds;
        let deleted = conn.execute(
            "DELETE FROM indications_history WHERE timestamp < ?1;",
            params![limit],
        )?;
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
            
            let desc_json: serde_json::Value = serde_json::from_str(&description).unwrap_or_else(|_| serde_json::json!(description));
            let val_json: serde_json::Value = value.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or(serde_json::Value::Null);
            
            let cemi_hex = to_hex(&cemi_raw);

            Ok(serde_json::json!({
                "id": id,
                "timestamp": timestamp,
                "group_address": group_address,
                "cemi_hex": cemi_hex,
                "description": desc_json,
                "value": val_json
            }))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn set_retention(&self, seconds: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO config_retention (key, value_seconds) VALUES ('retention', ?1);",
            params![seconds],
        )?;
        Ok(())
    }

    pub fn get_retention(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT value_seconds FROM config_retention WHERE key = 'retention';"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(604800)
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
