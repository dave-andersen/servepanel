use anyhow::Result;
use axum::{error_handling::HandleError, http::StatusCode, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeFile;

const DAILY_QUERY: &str = "select date, total_watt_hours from daily_view_fast_string";

const HOURLY_QUERY: &str = std::include_str!("hourly_query.sql");

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
    let (current, day_history, hour_history) = duckdb_power()?;
    Ok(Json(PowerResponse {
        current,
        day_history,
        hour_history,
    }))
}

pub async fn current_power() -> Result<Json<RawPowerLogEntry>> {
    let current = duckdb_current_power()?;
    Ok(Json(current))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RawPowerLogEntry {
    power: u32,
    time: String,
}
#[derive(Deserialize, Serialize, Debug)]
struct PowerLogEntry {
    power: u32,
    time: DateTime<Utc>,
}

fn duckdb_current_power() -> duckdb::Result<RawPowerLogEntry> {
    const DB: &str = "/home/dga/solar/solarpower.duckdb";
    let readonly_config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .unwrap();
    let conn = duckdb::Connection::open_with_flags(DB, readonly_config)?;

    Ok(conn
        .prepare("select power, strftime(time at time zone 'EST', '%Y-%m-%d %H:%M:%S') from (SELECT * from powerlog UNION ALL select * from solar_log_json) order by time desc limit 1")?
        .query_map([], |row| {
            Ok(RawPowerLogEntry {
                power: row.get(0)?,
                time: row.get(1)?,
            })
        })?
        .last()
        .ok_or(duckdb::Error::QueryReturnedNoRows)??)
}

fn duckdb_power() -> duckdb::Result<(RawPowerLogEntry, Vec<PowerTimeEntry>, Vec<PowerTimeEntry>)> {
    const DB: &str = "/home/dga/solar/solarpower.duckdb";
    let readonly_config = duckdb::Config::default()
        .access_mode(duckdb::AccessMode::ReadOnly)
        .unwrap();
    let conn = duckdb::Connection::open_with_flags(DB, readonly_config)?;

    let current_power = conn
        .prepare("select power, strftime(time at time zone 'EST', '%Y-%m-%d %H:%M:%S') from (SELECT * from powerlog UNION ALL select * from solar_log_json) order by time desc limit 1")?
        .query_map([], |row| {
            Ok(RawPowerLogEntry {
                power: row.get(0)?,
                time: row.get(1)?,
            })
        })?
        .last()
        .ok_or(duckdb::Error::QueryReturnedNoRows)??;

    let map_to_pte = |row: &duckdb::Row| -> duckdb::Result<PowerTimeEntry> {
        Ok(PowerTimeEntry {
            time: row.get(0)?,
            watt_hours: row.get(1)?,
        })
    };

    let day_history = conn
        .prepare(DAILY_QUERY)?
        .query_map([], map_to_pte)?
        .filter_map(Result::ok)
        .collect();

    let hour_history = conn
        .prepare(HOURLY_QUERY)?
        .query_map([], map_to_pte)?
        .filter_map(Result::ok)
        .collect();

    Ok((current_power, day_history, hour_history))
}

#[tokio::main]
async fn main() {
    use tokio::net::TcpListener;

    let power_service = tower::service_fn(|_req| async {
        let b = power().await?;
        Ok::<_, anyhow::Error>(b)
    });
    let current_power_service = tower::service_fn(|_req| async {
        let b = current_power().await?;
        Ok::<_, anyhow::Error>(b)
    });

    let app = Router::new()
        .route_service("/", ServeFile::new("servepage.html"))
        .route_service(
            "/power",
            HandleError::new(power_service, handle_anyhow_error),
        )
        .route_service(
            "/current",
            HandleError::new(current_power_service, handle_anyhow_error),
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
