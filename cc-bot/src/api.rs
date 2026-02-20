//! HTTP API - Discord外からの操作を可能にする

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, RwLock};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{debug, error, info, warn};

use crate::rate_limiter::RateLimiter;
use crate::security::mask_secrets;

/// APIキー認証ミドルウェア
async fn auth_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // API_KEYは必須（複数サーバー導入時のセキュリティ確保）
    let expected_api_key = std::env::var("API_KEY").unwrap_or_default();
    if expected_api_key.is_empty() {
        error!("API_KEY environment variable is not set - authentication is required");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Server configuration error: API_KEY not set".to_string(),
            }),
        ));
    }

    // Authorization headerをチェック
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(h) if h.starts_with("Bearer ") => {
            let token = &h[7..];
            // 定数時間比較でタイミング攻撃を防止
            if token.as_bytes().ct_eq(expected_api_key.as_bytes()).into() {
                Ok(next.run(request).await)
            } else {
                warn!("Invalid API key attempt");
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid API key".to_string(),
                    }),
                ))
            }
        }
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Missing or invalid Authorization header".to_string(),
            }),
        )),
    }
}

/// レートリミットミドルウェア
///
/// APIリクエストのレートリミットを行い、DoS攻撃を防止する。
/// APIキー + クライアントIPの組み合わせで識別。
async fn rate_limit_middleware(
    State(state): State<Arc<ApiState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // X-Forwarded-For ヘッダーからクライアントIPを取得（プロキシ経由の場合）
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // APIキー認証後なのでAPIキーハッシュを使用
    let api_key_hash = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            let mut hasher = DefaultHasher::new();
            h.hash(&mut hasher);
            hasher.finish()
        })
        .unwrap_or(0);

    // APIキー + IP の組み合わせでクライアントを一意に識別
    let mut hasher = DefaultHasher::new();
    api_key_hash.hash(&mut hasher);
    client_ip.hash(&mut hasher);
    let client_id = hasher.finish();

    let mut limiter = state.rate_limiter.lock().await;

    if !limiter.check(client_id) {
        let retry_after = limiter.retry_after(client_id).unwrap_or(60);
        warn!("API rate limit exceeded for client (ip={})", client_ip);

        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: format!("Rate limit exceeded. Retry after {} seconds.", retry_after),
            }),
        ));
    }

    limiter.record(client_id);
    drop(limiter); // ロックを解放

    Ok(next.run(request).await)
}

/// メッセージ長の最大値
const MAX_MESSAGE_LENGTH: usize = 4000;

/// 入力バリデーション
fn validate_message(message: &str) -> Result<(), String> {
    if message.is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    if message.len() > MAX_MESSAGE_LENGTH {
        return Err(format!(
            "Message too long (max {} characters)",
            MAX_MESSAGE_LENGTH
        ));
    }
    Ok(())
}

use crate::llm::LLMClient;
use crate::history::ChatMessage;
use crate::memory_store::MemoryStore;
use crate::scheduler::Scheduler;
use crate::schedule_store::ScheduleStore;
use crate::session::SessionManager;
use crate::tool::ToolContext;

/// APIサーバーの共有状態
#[derive(Clone)]
pub struct ApiState {
    pub glm_client: Arc<dyn LLMClient>,
    /// 将来的にセッション履歴APIで使用予定
    #[allow(dead_code)]
    pub session_manager: Arc<Mutex<SessionManager>>,
    pub scheduler: Arc<Scheduler>,
    pub schedule_store: Arc<RwLock<ScheduleStore>>,
    pub memory_store: Arc<MemoryStore>,
    pub base_output_dir: String,
    /// レートリミッター（DoS攻撃防止）
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
}

/// ヘルスチェックレスポンス
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// チャットリクエスト
#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub user_id: Option<u64>,
    #[serde(default)]
    pub channel_id: Option<u64>,
}

/// チャットレスポンス
#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
}

/// スケジュール作成リクエスト
#[derive(Deserialize)]
pub struct CreateScheduleRequest {
    pub cron: String,
    pub prompt: String,
    #[serde(default)]
    pub channel_id: u64,
}

/// スケジュールレスポンス
#[derive(Serialize)]
pub struct ScheduleResponse {
    pub id: String,
    pub cron: String,
    pub prompt: String,
    pub channel_id: u64,
    pub next_run: Option<String>,
}

/// メモリ作成リクエスト
#[derive(Deserialize)]
pub struct CreateMemoryRequest {
    pub user_id: u64,
    pub content: String,
}

/// メモリレスポンス
#[derive(Serialize)]
pub struct MemoryResponse {
    pub id: i64,
    pub user_id: u64,
    pub content: String,
    pub created_at: String,
}

/// メモリ検索クエリ
#[derive(Deserialize)]
pub struct SearchMemoryQuery {
    pub q: String,
    #[serde(default)]
    pub user_id: Option<u64>,
}

/// メモリ一覧クエリ
#[derive(Deserialize)]
pub struct ListMemoryQuery {
    #[serde(default)]
    pub user_id: Option<u64>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// エラーレスポンス
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

/// APIルーターを作成
pub fn create_router(state: ApiState) -> Router {
    // CORS設定 - 環境変数から許可するオリジンを取得
    // 本番環境ではALLOWED_ORIGINSの設定が必須
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .expect("ALLOWED_ORIGINS environment variable must be set for security. Example: ALLOWED_ORIGINS=https://example.com,https://app.example.com");

    if allowed_origins.trim().is_empty() {
        panic!("ALLOWED_ORIGINS cannot be empty. Please specify at least one allowed origin.");
    }

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .split(',')
                .map(|s| s.trim().parse::<axum::http::HeaderValue>())
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    // セキュリティヘッダー
    let security_headers = ServiceBuilder::new()
        // Content-Security-Policy: JSON API用の厳格なポリシー
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        ))
        // X-Content-Type-Options: nosniff
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        // X-Frame-Options: DENY
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        // X-XSS-Protection: 1; mode=block (legacy but still useful)
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        // Strict-Transport-Security: max-age=31536000; includeSubDomains
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ));

    Router::new()
        // ヘルスチェック（認証不要、レートリミットなし）
        .route("/api/health", get(health))
        // 認証が必要なルート
        .nest(
            "/api",
            Router::new()
                // チャット
                .route("/chat", post(chat))
                // スケジュール
                .route("/schedules", get(list_schedules).post(create_schedule))
                .route("/schedules/{id}", delete(delete_schedule))
                // メモリ
                .route("/memories", get(list_memories).post(create_memory))
                .route("/memories/search", get(search_memories))
                .route("/memories/{id}", delete(delete_memory))
                // 認証ミドルウェア
                .layer(middleware::from_fn(auth_middleware))
                // レートリミットミドルウェア
                .route_layer(middleware::from_fn_with_state(
                    Arc::new(state.clone()),
                    rate_limit_middleware,
                )),
        )
        .layer(security_headers)
        .layer(cors)
        .with_state(Arc::new(state))
}

// ===== ヘルスチェック =====

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ===== チャット =====

async fn chat(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 入力バリデーション
    if let Err(e) = validate_message(&req.message) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        ));
    }

    let user_id = req.user_id.unwrap_or(0);
    let channel_id = req.channel_id.unwrap_or(0);

    info!("API chat request from user {}: {}", user_id, mask_secrets(&req.message));

    // メッセージを作成
    let messages = vec![ChatMessage::user(req.message.clone())];

    // ツールコンテキスト
    let tool_context = ToolContext::new(
        user_id,
        "api".to_string(),
        channel_id,
        state.base_output_dir.clone(),
    );

    // GLM APIに問い合わせ
    match state.glm_client.chat_with_tools(messages, &tool_context).await {
        Ok(response) => Ok(Json(ChatResponse { response })),
        Err(e) => {
            error!("GLM API error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("GLM API error: {}", e),
                }),
            ))
        }
    }
}

// ===== スケジュール =====

async fn list_schedules(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<ScheduleResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.scheduler.list_tasks().await;

    let schedules: Vec<ScheduleResponse> = tasks
        .into_iter()
        .map(|t| {
            let next_run = t.next_run().map(|d| d.to_rfc3339());
            ScheduleResponse {
                id: t.id.to_string(),
                cron: t.cron_expression,
                prompt: t.prompt,
                channel_id: t.channel_id,
                next_run,
            }
        })
        .collect();

    Ok(Json(schedules))
}

async fn create_schedule(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    use crate::scheduler::ScheduledTask;

    match ScheduledTask::new(req.cron.clone(), req.prompt.clone(), req.channel_id) {
        Ok(task) => {
            let id = task.id;
            let next_run = task.next_run();

            // スケジューラーに追加
            state.scheduler.add_task(task.clone()).await;

            // ストアに保存
            {
                let mut store = state.schedule_store.write().await;
                store.add_task(task);
                if let Err(e) = store.save("data").await {
                    error!("Failed to save schedule: {}", e);
                }
            }

            Ok(Json(ScheduleResponse {
                id: id.to_string(),
                cron: req.cron,
                prompt: req.prompt,
                channel_id: req.channel_id,
                next_run: next_run.map(|d| d.to_rfc3339()),
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid schedule: {}", e),
            }),
        )),
    }
}

async fn delete_schedule(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            match state.scheduler.remove_task(uuid).await {
                Ok(_) => {
                    // ストアからも削除
                    {
                        let mut store = state.schedule_store.write().await;
                        store.remove_task(uuid);
                        if let Err(e) = store.save("data").await {
                            error!("Failed to save after remove: {}", e);
                        }
                    }
                    Ok(StatusCode::NO_CONTENT)
                }
                Err(e) => Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("Schedule not found: {}", e),
                    }),
                )),
            }
        }
        Err(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid UUID".to_string(),
            }),
        )),
    }
}

// ===== メモリ =====

async fn list_memories(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ListMemoryQuery>,
) -> Result<Json<Vec<MemoryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = query.user_id.unwrap_or(0);

    match state.memory_store.list_memories(user_id, query.limit) {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories
                .into_iter()
                .map(|m| MemoryResponse {
                    id: m.id,
                    user_id: m.user_id,
                    content: m.content,
                    created_at: m.created_at.to_rfc3339(),
                })
                .collect();
            Ok(Json(responses))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list memories: {}", e),
            }),
        )),
    }
}

async fn create_memory(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<Json<MemoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let new_memory = crate::memory_store::NewMemory {
        user_id: req.user_id,
        content: req.content,
        ..Default::default()
    };

    match state.memory_store.add_memory(new_memory) {
        Ok(memory) => Ok(Json(MemoryResponse {
            id: memory.id,
            user_id: memory.user_id,
            content: memory.content,
            created_at: memory.created_at.to_rfc3339(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create memory: {}", e),
            }),
        )),
    }
}

async fn search_memories(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<SearchMemoryQuery>,
) -> Result<Json<Vec<MemoryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = query.user_id.unwrap_or(0);

    match state.memory_store.search_memories(user_id, &query.q) {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories
                .into_iter()
                .map(|m| MemoryResponse {
                    id: m.id,
                    user_id: m.user_id,
                    content: m.content,
                    created_at: m.created_at.to_rfc3339(),
                })
                .collect();
            Ok(Json(responses))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to search memories: {}", e),
            }),
        )),
    }
}

async fn delete_memory(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // APIでは user_id は0（システム）として扱う
    match state.memory_store.delete_memory(0, id) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Memory not found: {}", e),
            }),
        )),
    }
}

/// APIサーバーを起動
pub async fn start_server(state: ApiState, port: u16) {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);

    info!("API server starting on {}", addr);

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("API server error: {}", e);
    }
}
