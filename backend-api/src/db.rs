use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Alert {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub tipo_evento: String,
    pub confianca: f32,
}

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    connect_with_max_connections(database_url, 5).await
}

async fn connect_with_max_connections(database_url: &str, max_connections: u32) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            tipo_evento TEXT NOT NULL,
            confianca REAL NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn insert_alert(pool: &SqlitePool, tipo_evento: &str, confianca: f32) -> Result<()> {
    insert_alert_at(pool, Utc::now(), tipo_evento, confianca).await
}

pub async fn insert_alert_at(
    pool: &SqlitePool,
    timestamp: DateTime<Utc>,
    tipo_evento: &str,
    confianca: f32,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO alerts (timestamp, tipo_evento, confianca)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(timestamp.to_rfc3339())
    .bind(tipo_evento)
    .bind(confianca)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_alerts(pool: &SqlitePool) -> Result<Vec<Alert>> {
    let alerts = sqlx::query_as::<_, Alert>(
        r#"
        SELECT id, timestamp, tipo_evento, confianca
        FROM alerts
        ORDER BY timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(alerts)
}

pub async fn count_alerts(pool: &SqlitePool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alerts")
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn seed_simulation_alerts(pool: &SqlitePool) -> Result<usize> {
    if count_alerts(pool).await? > 0 {
        return Ok(0);
    }

    let now = Utc::now();
    let samples = [
        (now - Duration::minutes(3), "motosserra", 96.0),
        (now - Duration::minutes(11), "tiro", 91.0),
        (now - Duration::minutes(24), "motosserra", 88.0),
        (now - Duration::minutes(52), "tiro", 93.0),
        (now - Duration::minutes(66), "motosserra", 85.0),
        (now - Duration::minutes(88), "tiro", 89.0),
    ];

    for (timestamp, tipo_evento, confianca) in samples {
        insert_alert_at(pool, timestamp, tipo_evento, confianca).await?;
    }

    Ok(samples.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_list_alerts_returns_latest_first() {
        let pool = connect_with_max_connections("sqlite::memory:", 1)
            .await
            .expect("in-memory database should be created");

        insert_alert(&pool, "motosserra", 94.0)
            .await
            .expect("first alert should be inserted");
        insert_alert(&pool, "tiro", 91.0)
            .await
            .expect("second alert should be inserted");

        let alerts = list_alerts(&pool)
            .await
            .expect("alerts should be listed");

        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].tipo_evento, "tiro");
        assert_eq!(alerts[0].confianca, 91.0);
        assert_eq!(alerts[1].tipo_evento, "motosserra");
        assert_eq!(alerts[1].confianca, 94.0);
    }

    #[tokio::test]
    async fn seed_simulation_alerts_populates_empty_database_once() {
        let pool = connect_with_max_connections("sqlite::memory:", 1)
            .await
            .expect("in-memory database should be created");

        let inserted = seed_simulation_alerts(&pool)
            .await
            .expect("simulation seed should succeed");
        let alerts = list_alerts(&pool)
            .await
            .expect("alerts should be listed");
        let inserted_again = seed_simulation_alerts(&pool)
            .await
            .expect("second simulation seed should succeed");

        assert_eq!(inserted, 6);
        assert_eq!(alerts.len(), 6);
        assert_eq!(inserted_again, 0);
    }
}
