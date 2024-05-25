use anyhow::{anyhow, Result};
use axum::{error_handling::HandleError, http::StatusCode, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use tower_http::services::ServeFile;

const LOGPATH: &str = "/home/dga/solar/log.json";

const DAILY_QUERY: &str = "select date, total_watt_hours from daily_view_fast_string";
// std::include_str!("daily_query.sql");

const HOURLY_QUERY: &str = std::include_str!("hourly_query.sql");

pub fn read_last_line(filename: &str) -> Result<String> {
    // Read the last line of the file, efficiently
    let mut file = File::open(filename)?;
    let file_size_bytes = file.metadata()?.len();
    let seekpos = if file_size_bytes < 1024 {
        0
    } else {
        file_size_bytes - 1024
    };
    file.seek(SeekFrom::Start(seekpos)).unwrap();
    let reader = BufReader::new(file);
    Ok(reader.lines().last().ok_or(anyhow!("no lines"))??)
}

#[derive(Deserialize, Serialize, Debug)]
struct PowerTimeEntry {
    time: String,
    watt_hours: f32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PowerResponse {
    current: RawPowerLogEntry,
    day_history: Vec<PowerTimeEntry>,
    hour_history: Vec<PowerTimeEntry>,
}

pub async fn power() -> Result<Json<PowerResponse>> {
    let lastlog = read_last_line(LOGPATH)?;
    let current: RawPowerLogEntry = serde_json::from_str(&lastlog)?;
    let (day_history, hour_history) = duckdb_power()?;
    Ok(Json(PowerResponse {
        current,
        day_history,
        hour_history,
    }))
}

#[derive(Deserialize, Serialize, Debug)]
struct RawPowerLogEntry {
    power: u32,
    time: String,
}
#[derive(Deserialize, Serialize, Debug)]
struct PowerLogEntry {
    power: u32,
    time: DateTime<Utc>,
}

fn duckdb_power() -> duckdb::Result<(Vec<PowerTimeEntry>, Vec<PowerTimeEntry>)> {
    const DB: &str = "/home/dga/solar/solarpower.duckdb";
    let readonly_config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .unwrap();
    let conn = duckdb::Connection::open_with_flags(DB, readonly_config)?;
    let mut stmt = conn.prepare(DAILY_QUERY)?;

    let map_to_pte = |row: &duckdb::Row| -> duckdb::Result<PowerTimeEntry> {
        Ok(PowerTimeEntry {
            time: row.get(0)?,
            watt_hours: row.get(1)?,
        })
    };

    let res = stmt.query_map([], map_to_pte)?;

    let day_history = res.filter_map(Result::ok).collect();

    let mut stmt = conn.prepare(HOURLY_QUERY)?;
    let res = stmt.query_map([], map_to_pte)?;

    let hour_history = res.filter_map(Result::ok).collect();

    Ok((day_history, hour_history))
}

#[tokio::main]
async fn main() {
    use tokio::net::TcpListener;

    let power_service = tower::service_fn(|_req| async {
        let b = power().await?;
        Ok::<_, anyhow::Error>(b)
    });

    let app = Router::new()
        .route_service("/", ServeFile::new("servepage.html"))
        .route_service(
            "/power",
            HandleError::new(power_service, handle_anyhow_error),
        );

    let listener = TcpListener::bind("0.0.0.0:8084").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_anyhow_error(err: anyhow::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Something went wrong: {err}"),
    )
}
