mod db;
mod inference;

use std::{env, net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use inference::{InferenceEngine, InferenceResult};
use serde::Serialize;
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    inference: Arc<InferenceEngine>,
}

#[derive(Serialize)]
struct AnalyzeResponse {
    label: String,
    confidence: f32,
    persisted: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/alerts.db".to_string());
    let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| "model.onnx".to_string());
    let simulation = simulation_enabled();
    cloud_log(format!(
        "Inicializando backend em 0.0.0.0:8080 com banco `{database_url}` e modelo `{model_path}`."
    ));
    let pool = db::connect(&database_url).await?;

    if simulation {
        let inserted = db::seed_simulation_alerts(&pool).await?;
        if inserted > 0 {
            cloud_log(format!(
                "Modo de simulação habilitado. {inserted} alerta(s) de exemplo foram carregados."
            ));
        } else {
            cloud_log(
                "Modo de simulação habilitado. O banco já possui alertas; nenhum dado adicional foi inserido.",
            );
        }
    }

    let state = AppState {
        db: pool,
        inference: Arc::new(InferenceEngine::new(model_path)),
    };

    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://localhost:5173"))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/analyze", post(analyze))
        .route("/alerts", get(alerts))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    cloud_log("API pronta para receber análises de áudio.");
    axum::serve(listener, app).await?;

    Ok(())
}

fn simulation_enabled() -> bool {
    env::var("SIMULATION")
        .map(|value| is_simulation_value(&value))
        .unwrap_or(false)
}

fn is_simulation_value(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

async fn analyze(State(state): State<AppState>, mut multipart: Multipart) -> impl IntoResponse {
    match read_file_field(&mut multipart).await {
        Ok(bytes) => {
            cloud_log(format!(
                "Áudio recebido para análise ({} bytes). Processando inferência...",
                bytes.len()
            ));
            match state.inference.analyze(&bytes) {
                Ok(result) => handle_inference_result(&state, result).await,
                Err(err) => {
                    cloud_error(format!("Falha ao processar áudio: {err}"));
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(serde_json::json!({ "error": err.to_string() })),
                    )
                        .into_response()
                }
            }
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn alerts(State(state): State<AppState>) -> impl IntoResponse {
    match db::list_alerts(&state.db).await {
        Ok(items) => {
            cloud_log(format!("Consulta de alertas concluída: {} registro(s).", items.len()));
            Json(items).into_response()
        }
        Err(err) => {
            cloud_error(format!("Falha ao listar alertas: {err}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response()
        }
    }
}

async fn read_file_field(multipart: &mut Multipart) -> Result<Vec<u8>, String> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| err.to_string())?
    {
        if field.name() == Some("file") {
            return field.bytes().await.map(|bytes| bytes.to_vec()).map_err(|err| err.to_string());
        }
    }

    Err("campo `file` não enviado".to_string())
}

async fn handle_inference_result(state: &AppState, result: InferenceResult) -> axum::response::Response {
    let should_persist = matches!(result.label.as_str(), "motosserra" | "tiro");
    let confidence_pct = result.confidence * 100.0;

    cloud_log(format!(
        "Som classificado: {} detectado com {:.0}% de confiança.",
        format_event_label(&result.label),
        confidence_pct
    ));

    if should_persist {
        if let Err(err) = db::insert_alert(&state.db, &result.label, confidence_pct).await {
            cloud_error(format!(
                "Falha ao persistir alerta de {}: {err}",
                format_event_label(&result.label)
            ));
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response();
        }

        cloud_log(format!(
            "Alerta enviado: {} detectada com {:.0}% de confiança.",
            format_event_label(&result.label),
            confidence_pct
        ));
    }

    (
        StatusCode::OK,
        Json(AnalyzeResponse {
            label: result.label,
            confidence: confidence_pct,
            persisted: should_persist,
        }),
    )
        .into_response()
}

fn format_event_label(label: &str) -> &'static str {
    match label {
        "motosserra" => "Motosserra",
        "tiro" => "Tiro",
        "chuva" => "Chuva",
        "ambiente" => "Som ambiente",
        _ => "Som desconhecido",
    }
}

fn cloud_log(message: impl AsRef<str>) {
    println!("[Curupira-Cloud] {}", message.as_ref());
}

fn cloud_error(message: impl AsRef<str>) {
    eprintln!("[Curupira-Cloud] {}", message.as_ref());
}

#[cfg(test)]
mod tests {
    use super::{format_event_label, is_simulation_value};

    #[test]
    fn format_event_label_maps_known_and_unknown_labels() {
        assert_eq!(format_event_label("motosserra"), "Motosserra");
        assert_eq!(format_event_label("ambiente"), "Som ambiente");
        assert_eq!(format_event_label("algo-novo"), "Som desconhecido");
    }

    #[test]
    fn simulation_value_parser_accepts_expected_truthy_inputs() {
        assert!(is_simulation_value("1"));
        assert!(is_simulation_value("true"));
        assert!(is_simulation_value("TRUE"));
        assert!(is_simulation_value("yes"));
        assert!(is_simulation_value("on"));
        assert!(!is_simulation_value("false"));
        assert!(!is_simulation_value("0"));
    }
}
