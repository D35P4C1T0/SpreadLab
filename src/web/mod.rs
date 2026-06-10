use crate::api::{
    calculate_damage_request, find_min_combined_hp_def_survival, find_min_hp_def_survival,
    find_min_offensive_ko, load_metadata, run_defensive_optimization, run_offensive_optimization,
    ApiError, CombinedHpDefSurvivalRequest, DamageRequest, HpDefSurvivalRequest,
    OffensiveKoRequest, OptimizeRequest,
};
use axum::extract::Json;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

pub fn run_blocking(config: ServeConfig) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spreadlab_rs=info,tower_http=info".into()),
        )
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(config))
}

async fn serve(config: ServeConfig) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let app = Router::new()
        .route("/", get(index))
        .route("/app.css", get(css))
        .route("/app.js", get(js))
        .route("/api/meta", get(meta))
        .route("/api/damage", post(damage))
        .route("/api/survive", post(survive))
        .route("/api/survive-sequence", post(survive_sequence))
        .route("/api/ko", post(ko))
        .route("/api/optimize/defensive", post(optimize_defensive))
        .route("/api/optimize/offensive", post(optimize_offensive))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("webui listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

async fn css() -> Response {
    static_response("text/css; charset=utf-8", include_str!("static/app.css"))
}

async fn js() -> Response {
    static_response(
        "application/javascript; charset=utf-8",
        include_str!("static/app.js"),
    )
}

async fn meta() -> Result<Json<impl Serialize>, WebError> {
    blocking(load_metadata).await.map(Json)
}

async fn damage(Json(request): Json<DamageRequest>) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || calculate_damage_request(request))
        .await
        .map(Json)
}

async fn survive(
    Json(request): Json<HpDefSurvivalRequest>,
) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || find_min_hp_def_survival(request))
        .await
        .map(Json)
}

async fn survive_sequence(
    Json(request): Json<CombinedHpDefSurvivalRequest>,
) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || find_min_combined_hp_def_survival(request))
        .await
        .map(Json)
}

async fn ko(Json(request): Json<OffensiveKoRequest>) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || find_min_offensive_ko(request))
        .await
        .map(Json)
}

async fn optimize_defensive(
    Json(request): Json<OptimizeRequest>,
) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || run_defensive_optimization(request))
        .await
        .map(Json)
}

async fn optimize_offensive(
    Json(request): Json<OptimizeRequest>,
) -> Result<Json<impl Serialize>, WebError> {
    blocking(move || run_offensive_optimization(request))
        .await
        .map(Json)
}

async fn blocking<T>(
    task: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, WebError>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|error| WebError::internal(error.to_string()))?
        .map_err(WebError::bad_request)
}

fn static_response(content_type: &'static str, body: &'static str) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

#[derive(Debug)]
struct WebError {
    status: StatusCode,
    message: String,
}

impl WebError {
    fn bad_request(error: ApiError) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorBody {
            error: String,
        }

        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}
