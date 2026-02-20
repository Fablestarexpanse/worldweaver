// ── terrain/persistence.rs ────────────────────────────────────────────────────
// Save / load world state using SQLite + zstd-compressed blobs.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use crate::state::TerrainData;
use super::config::TerrainConfig;

/// Save a world to a `.wwdb` SQLite file.
pub fn save(path: &str, data: &TerrainData) -> Result<()> {
    let conn = Connection::open(path)
        .with_context(|| format!("open db: {path}"))?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS blobs (
            name    TEXT PRIMARY KEY,
            data    BLOB NOT NULL
        );
    ")?;

    // Serialise config as JSON
    let cfg_json = serde_json::to_string(&data.config)?;
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES('config', ?1)",
        params![cfg_json],
    )?;

    // Compress and store heights
    let heights_raw: &[u8] = bytemuck::cast_slice(&data.heights);
    let heights_compressed = zstd::encode_all(heights_raw, 6)?;
    conn.execute(
        "INSERT OR REPLACE INTO blobs(name, data) VALUES('heights', ?1)",
        params![heights_compressed],
    )?;

    // Compress and store flow
    let flow_raw: &[u8] = bytemuck::cast_slice(&data.flow);
    let flow_compressed = zstd::encode_all(flow_raw, 6)?;
    conn.execute(
        "INSERT OR REPLACE INTO blobs(name, data) VALUES('flow', ?1)",
        params![flow_compressed],
    )?;

    // Store biomes (already u8, low entropy — zstd still helps)
    let biomes_compressed = zstd::encode_all(data.biomes.as_slice(), 6)?;
    conn.execute(
        "INSERT OR REPLACE INTO blobs(name, data) VALUES('biomes', ?1)",
        params![biomes_compressed],
    )?;

    log::info!("World saved to {path}");
    Ok(())
}

/// Load a world from a `.wwdb` SQLite file.
pub fn load(path: &str) -> Result<TerrainData> {
    let conn = Connection::open(path)
        .with_context(|| format!("open db: {path}"))?;

    // Load config
    let cfg_json: String = conn.query_row(
        "SELECT value FROM meta WHERE key='config'",
        [],
        |row| row.get(0),
    )?;
    let config: TerrainConfig = serde_json::from_str(&cfg_json)?;

    // Load and decompress heights
    let heights_compressed: Vec<u8> = conn.query_row(
        "SELECT data FROM blobs WHERE name='heights'",
        [],
        |row| row.get(0),
    )?;
    let heights_raw = zstd::decode_all(heights_compressed.as_slice())?;
    let heights: Vec<f32> = bytemuck::cast_slice(&heights_raw).to_vec();

    // Load and decompress flow
    let flow_compressed: Vec<u8> = conn.query_row(
        "SELECT data FROM blobs WHERE name='flow'",
        [],
        |row| row.get(0),
    )?;
    let flow_raw = zstd::decode_all(flow_compressed.as_slice())?;
    let flow: Vec<f32> = bytemuck::cast_slice(&flow_raw).to_vec();

    // Load and decompress biomes
    let biomes_compressed: Vec<u8> = conn.query_row(
        "SELECT data FROM blobs WHERE name='biomes'",
        [],
        |row| row.get(0),
    )?;
    let biomes = zstd::decode_all(biomes_compressed.as_slice())?;

    log::info!("World loaded from {path}");
    Ok(TerrainData { config, heights, flow, biomes, dirty: true })
}
