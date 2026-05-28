use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, params};
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub fn db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
        .join("cld");

    fs::create_dir_all(&data_dir)?;

    Ok(data_dir.join("cld.db"))
}

pub fn connect() -> Result<Connection> {
    let path = db_path()?;

    println!("Database path: {}", path.display());

    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS messages (
            id        TEXT NOT NULL PRIMARY KEY,
            peer      TEXT NOT NULL,
            direction TEXT NOT NULL,
            content   TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_messages_peer
        ON messages(peer, timestamp);

        CREATE TABLE IF NOT EXISTS contacts (
            name    TEXT NOT NULL PRIMARY KEY,
            address TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS peer_keys (
            peer_name   TEXT NOT NULL PRIMARY KEY,
            fingerprint TEXT NOT NULL,
            first_seen  INTEGER NOT NULL,
            last_seen   INTEGER NOT NULL
        );
        ",
    )?;

    Ok(())
}

pub fn insert_message(
    conn: &Connection,
    id: Uuid,
    peer: &str,
    direction: &str,
    content: &str,
    timestamp: i64,
) -> Result<()> {
    conn.execute(
        "
        INSERT INTO messages (id, peer, direction, content, timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![id.to_string(), peer, direction, content, timestamp],
    )?;

    Ok(())
}

pub fn get_peer_fingerprint(conn: &Connection, peer_name: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "
        SELECT fingerprint
        FROM peer_keys
        WHERE peer_name = ?1
        ",
    )?;

    let result = stmt.query_row(params![peer_name], |row| row.get::<_, String>(0));

    match result {
        Ok(fingerprint) => Ok(Some(fingerprint)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn store_peer_fingerprint(conn: &Connection, peer_name: &str, fingerprint: &str) -> Result<()> {
    let now = Utc::now().timestamp();

    conn.execute(
        "
        INSERT INTO peer_keys (peer_name, fingerprint, first_seen, last_seen)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(peer_name)
        DO UPDATE SET last_seen = excluded.last_seen
        ",
        params![peer_name, fingerprint, now, now],
    )?;

    Ok(())
}

pub fn verify_or_store_peer_fingerprint(
    conn: &Connection,
    peer_name: &str,
    fingerprint: &str,
) -> Result<bool> {
    match get_peer_fingerprint(conn, peer_name)? {
        Some(stored) => Ok(stored == fingerprint),
        None => {
            store_peer_fingerprint(conn, peer_name, fingerprint)?;
            Ok(true)
        }
    }
}
