use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Json, Path, Query, Request, State},
    http::{header::HeaderValue, header::AUTHORIZATION, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    env,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::warn;

#[derive(Clone, Debug)]
pub struct PanelConfig {
    pub bind_addr: SocketAddr,
    pub jwt_secret: String,
    pub jwt_ttl_seconds: u64,
    pub admin_username: String,
    pub admin_password: String,
    pub cors_allowed_origins: Vec<String>,
    pub rate_limit_rps: u64,
}

impl PanelConfig {
    pub fn from_env() -> Result<Self> {
        let bind_addr = read_env("VELOPANEL_BIND", "0.0.0.0:7070")?
            .parse::<SocketAddr>()
            .context("failed to parse VELOPANEL_BIND as socket address")?;

        let jwt_secret = read_env("VELOPANEL_JWT_SECRET", "velopanel-dev-secret-change-me")?;
        if jwt_secret == "velopanel-dev-secret-change-me" {
            warn!("VELOPANEL_JWT_SECRET not set, using development default");
        }

        let admin_username = read_env("VELOPANEL_ADMIN_USERNAME", "admin")?;
        let admin_password = read_env("VELOPANEL_ADMIN_PASSWORD", "admin")?;
        if admin_password == "admin" {
            warn!("VELOPANEL_ADMIN_PASSWORD not set, using insecure development default");
        }

        let cors_raw = read_env("VELOPANEL_CORS_ORIGINS", "*")?;
        let cors_allowed_origins = cors_raw
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        let jwt_ttl_seconds = read_env_u64("VELOPANEL_JWT_TTL_SECONDS", 24 * 60 * 60)?;
        let rate_limit_rps = read_env_u64("VELOPANEL_RATE_LIMIT_RPS", 50)?;

        Ok(Self {
            bind_addr,
            jwt_secret,
            jwt_ttl_seconds,
            admin_username,
            admin_password,
            cors_allowed_origins,
            rate_limit_rps,
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<PanelConfig>,
    limiter: Arc<RateLimiter>,
    databases: Arc<DatabaseModule>,
}

impl AppState {
    pub fn new(config: PanelConfig) -> Self {
        let rate_limit_rps = config.rate_limit_rps.max(1);
        Self {
            config: Arc::new(config),
            limiter: Arc::new(RateLimiter::new(rate_limit_rps)),
            databases: Arc::new(DatabaseModule::new()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

#[derive(Debug, Serialize)]
struct ApiEnvelope<T> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum DatabaseEngine {
    Mysql,
    Mariadb,
}

#[derive(Debug, Deserialize)]
struct CreateDatabaseRequest {
    account_id: String,
    name: String,
    engine: DatabaseEngine,
}

#[derive(Debug, Deserialize)]
struct ListDatabasesQuery {
    account_id: String,
    engine: Option<DatabaseEngine>,
}

#[derive(Debug, Deserialize)]
struct DeleteDatabaseQuery {
    account_id: String,
    engine: DatabaseEngine,
}

#[derive(Debug, Deserialize)]
struct CreateDatabaseUserRequest {
    account_id: String,
    username: String,
    password: String,
    engine: DatabaseEngine,
}

#[derive(Debug, Deserialize)]
struct ListDatabaseUsersQuery {
    account_id: String,
    engine: Option<DatabaseEngine>,
}

#[derive(Debug, Deserialize)]
struct DeleteDatabaseUserQuery {
    account_id: String,
    engine: DatabaseEngine,
}

#[derive(Debug, Deserialize)]
struct RotateDatabaseUserPasswordRequest {
    account_id: String,
    new_password: String,
    engine: DatabaseEngine,
}

#[derive(Debug, Deserialize)]
struct UpdateDatabaseGrantRequest {
    account_id: String,
    username: String,
    database: String,
    engine: DatabaseEngine,
}

#[derive(Clone, Debug, Serialize)]
struct DatabaseSummary {
    name: String,
    account_id: String,
    engine: DatabaseEngine,
    granted_users: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DatabaseUserSummary {
    username: String,
    account_id: String,
    engine: DatabaseEngine,
    password_version: u64,
    granted_databases: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DatabaseGrantSummary {
    account_id: String,
    username: String,
    database: String,
    engine: DatabaseEngine,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

type DbResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }
}

#[derive(Default)]
struct DatabaseStore {
    databases: HashMap<(DatabaseEngine, String), DatabaseRecord>,
    users: HashMap<(DatabaseEngine, String), DatabaseUserRecord>,
}

#[derive(Clone)]
struct DatabaseRecord {
    name: String,
    account_id: String,
    engine: DatabaseEngine,
    users: HashSet<String>,
}

#[derive(Clone)]
struct DatabaseUserRecord {
    username: String,
    account_id: String,
    engine: DatabaseEngine,
    password_version: u64,
    databases: HashSet<String>,
}

struct DatabaseModule {
    store: Mutex<DatabaseStore>,
}

impl DatabaseModule {
    fn new() -> Self {
        Self {
            store: Mutex::new(DatabaseStore::default()),
        }
    }

    fn list_databases(
        &self,
        account_id: &str,
        engine: Option<DatabaseEngine>,
    ) -> DbResult<Vec<DatabaseSummary>> {
        validate_account_id(account_id)?;
        let store = self.store.lock().expect("database module mutex poisoned");
        let mut items = store
            .databases
            .values()
            .filter(|db| db.account_id == account_id)
            .filter(|db| engine.is_none_or(|selected| selected == db.engine))
            .map(Self::database_summary)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(items)
    }

    fn create_database(&self, req: CreateDatabaseRequest) -> DbResult<DatabaseSummary> {
        validate_account_id(&req.account_id)?;
        validate_database_name(&req.name)?;
        ensure_account_prefix(&req.account_id, &req.name, "database name")?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let key = (req.engine, req.name.clone());
        if store.databases.contains_key(&key) {
            return Err(ApiError::conflict("database already exists"));
        }

        let record = DatabaseRecord {
            name: req.name,
            account_id: req.account_id,
            engine: req.engine,
            users: HashSet::new(),
        };
        let summary = Self::database_summary(&record);
        store.databases.insert(key, record);
        Ok(summary)
    }

    fn delete_database(
        &self,
        account_id: &str,
        name: &str,
        engine: DatabaseEngine,
    ) -> DbResult<DatabaseSummary> {
        validate_account_id(account_id)?;
        validate_database_name(name)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let key = (engine, name.to_string());

        let record = store
            .databases
            .get(&key)
            .cloned()
            .ok_or_else(|| ApiError::not_found("database not found"))?;

        if record.account_id != account_id {
            return Err(ApiError::forbidden(
                "cross-account database access is not allowed",
            ));
        }

        store.databases.remove(&key);
        for username in &record.users {
            if let Some(user) = store.users.get_mut(&(engine, username.clone())) {
                user.databases.remove(name);
            }
        }

        Ok(Self::database_summary(&record))
    }

    fn list_users(
        &self,
        account_id: &str,
        engine: Option<DatabaseEngine>,
    ) -> DbResult<Vec<DatabaseUserSummary>> {
        validate_account_id(account_id)?;
        let store = self.store.lock().expect("database module mutex poisoned");
        let mut items = store
            .users
            .values()
            .filter(|user| user.account_id == account_id)
            .filter(|user| engine.is_none_or(|selected| selected == user.engine))
            .map(Self::user_summary)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(items)
    }

    fn create_user(&self, req: CreateDatabaseUserRequest) -> DbResult<DatabaseUserSummary> {
        validate_account_id(&req.account_id)?;
        validate_db_username(&req.username)?;
        ensure_account_prefix(&req.account_id, &req.username, "database username")?;
        validate_password(&req.password)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let key = (req.engine, req.username.clone());
        if store.users.contains_key(&key) {
            return Err(ApiError::conflict("database user already exists"));
        }

        let user = DatabaseUserRecord {
            username: req.username,
            account_id: req.account_id,
            engine: req.engine,
            password_version: 1,
            databases: HashSet::new(),
        };
        let summary = Self::user_summary(&user);
        store.users.insert(key, user);
        Ok(summary)
    }

    fn rotate_user_password(
        &self,
        account_id: &str,
        username: &str,
        new_password: &str,
        engine: DatabaseEngine,
    ) -> DbResult<DatabaseUserSummary> {
        validate_account_id(account_id)?;
        validate_db_username(username)?;
        validate_password(new_password)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let key = (engine, username.to_string());
        let user = store
            .users
            .get_mut(&key)
            .ok_or_else(|| ApiError::not_found("database user not found"))?;

        if user.account_id != account_id {
            return Err(ApiError::forbidden(
                "cross-account database user access is not allowed",
            ));
        }

        user.password_version = user.password_version.saturating_add(1);
        Ok(Self::user_summary(user))
    }

    fn delete_user(
        &self,
        account_id: &str,
        username: &str,
        engine: DatabaseEngine,
    ) -> DbResult<DatabaseUserSummary> {
        validate_account_id(account_id)?;
        validate_db_username(username)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let key = (engine, username.to_string());

        let user = store
            .users
            .get(&key)
            .cloned()
            .ok_or_else(|| ApiError::not_found("database user not found"))?;

        if user.account_id != account_id {
            return Err(ApiError::forbidden(
                "cross-account database user access is not allowed",
            ));
        }

        store.users.remove(&key);
        for database in &user.databases {
            if let Some(record) = store.databases.get_mut(&(engine, database.clone())) {
                record.users.remove(username);
            }
        }

        Ok(Self::user_summary(&user))
    }

    fn grant_access(&self, req: UpdateDatabaseGrantRequest) -> DbResult<DatabaseGrantSummary> {
        validate_account_id(&req.account_id)?;
        validate_db_username(&req.username)?;
        validate_database_name(&req.database)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let user_key = (req.engine, req.username.clone());
        let db_key = (req.engine, req.database.clone());

        let user_account = store
            .users
            .get(&user_key)
            .map(|user| user.account_id.clone())
            .ok_or_else(|| ApiError::not_found("database user not found"))?;

        if user_account != req.account_id {
            return Err(ApiError::forbidden(
                "cross-account database user access is not allowed",
            ));
        }

        let db_account = store
            .databases
            .get(&db_key)
            .map(|db| db.account_id.clone())
            .ok_or_else(|| ApiError::not_found("database not found"))?;

        if db_account != req.account_id {
            return Err(ApiError::forbidden(
                "cross-account database access is not allowed",
            ));
        }

        if let Some(user) = store.users.get_mut(&user_key) {
            user.databases.insert(req.database.clone());
        }

        if let Some(db) = store.databases.get_mut(&db_key) {
            db.users.insert(req.username.clone());
        }

        Ok(DatabaseGrantSummary {
            account_id: req.account_id,
            username: req.username,
            database: req.database,
            engine: req.engine,
        })
    }

    fn revoke_access(&self, req: UpdateDatabaseGrantRequest) -> DbResult<DatabaseGrantSummary> {
        validate_account_id(&req.account_id)?;
        validate_db_username(&req.username)?;
        validate_database_name(&req.database)?;

        let mut store = self.store.lock().expect("database module mutex poisoned");
        let user_key = (req.engine, req.username.clone());
        let db_key = (req.engine, req.database.clone());

        let user_account = store
            .users
            .get(&user_key)
            .map(|user| user.account_id.clone())
            .ok_or_else(|| ApiError::not_found("database user not found"))?;

        if user_account != req.account_id {
            return Err(ApiError::forbidden(
                "cross-account database user access is not allowed",
            ));
        }

        let db_account = store
            .databases
            .get(&db_key)
            .map(|db| db.account_id.clone())
            .ok_or_else(|| ApiError::not_found("database not found"))?;

        if db_account != req.account_id {
            return Err(ApiError::forbidden(
                "cross-account database access is not allowed",
            ));
        }

        if let Some(user) = store.users.get_mut(&user_key) {
            user.databases.remove(&req.database);
        }
        if let Some(db) = store.databases.get_mut(&db_key) {
            db.users.remove(&req.username);
        }

        Ok(DatabaseGrantSummary {
            account_id: req.account_id,
            username: req.username,
            database: req.database,
            engine: req.engine,
        })
    }

    fn database_summary(record: &DatabaseRecord) -> DatabaseSummary {
        DatabaseSummary {
            name: record.name.clone(),
            account_id: record.account_id.clone(),
            engine: record.engine,
            granted_users: record.users.len(),
        }
    }

    fn user_summary(record: &DatabaseUserRecord) -> DatabaseUserSummary {
        DatabaseUserSummary {
            username: record.username.clone(),
            account_id: record.account_id.clone(),
            engine: record.engine,
            password_version: record.password_version,
            granted_databases: record.databases.len(),
        }
    }
}

pub fn build_router(state: AppState) -> Result<Router> {
    let cors = cors_layer(&state.config)?;

    let protected = Router::new()
        .route("/api/auth/me", get(auth_me))
        .route("/api/protected/ping", get(protected_ping))
        .route("/api/databases", get(list_databases).post(create_database))
        .route("/api/databases/:name", delete(delete_database))
        .route(
            "/api/database-users",
            get(list_database_users).post(create_database_user),
        )
        .route(
            "/api/database-users/:username",
            delete(delete_database_user),
        )
        .route(
            "/api/database-users/:username/rotate-password",
            post(rotate_database_user_password),
        )
        .route("/api/database-grants", post(grant_database_access))
        .route("/api/database-grants/revoke", post(revoke_database_access))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(api_health))
        .route("/api/version", get(version))
        .route("/api/auth/login", post(login))
        .merge(protected)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    Ok(app)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn api_health() -> impl IntoResponse {
    ok_response(json!({
        "status": "ok",
    }))
}

async fn version() -> impl IntoResponse {
    ok_response(json!({
        "service": "velopanel-api",
        "version": crate::VERSION,
    }))
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    if req.username != state.config.admin_username || req.password != state.config.admin_password {
        return error_response(StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    match issue_token(&state.config, &req.username) {
        Ok(token) => ok_response(json!({
            "token": token,
            "token_type": "Bearer",
            "expires_in_seconds": state.config.jwt_ttl_seconds,
        }))
        .into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to issue token: {err}"),
        )
        .into_response(),
    }
}

async fn auth_me(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    ok_response(json!({
        "username": claims.sub,
        "role": claims.role,
    }))
}

async fn protected_ping(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    ok_response(json!({
        "message": "pong",
        "user": claims.sub,
    }))
}

async fn list_databases(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Query(query): Query<ListDatabasesQuery>,
) -> Response {
    match state
        .databases
        .list_databases(&query.account_id, query.engine)
    {
        Ok(databases) => ok_response(json!({ "items": databases })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn create_database(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<CreateDatabaseRequest>,
) -> Response {
    match state.databases.create_database(req) {
        Ok(database) => ok_response(json!({ "database": database })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn delete_database(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<DeleteDatabaseQuery>,
) -> Response {
    match state
        .databases
        .delete_database(&query.account_id, &name, query.engine)
    {
        Ok(database) => ok_response(json!({ "database": database })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn list_database_users(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Query(query): Query<ListDatabaseUsersQuery>,
) -> Response {
    match state.databases.list_users(&query.account_id, query.engine) {
        Ok(users) => ok_response(json!({ "items": users })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn create_database_user(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<CreateDatabaseUserRequest>,
) -> Response {
    match state.databases.create_user(req) {
        Ok(user) => ok_response(json!({ "user": user })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn rotate_database_user_password(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Path(username): Path<String>,
    Json(req): Json<RotateDatabaseUserPasswordRequest>,
) -> Response {
    match state.databases.rotate_user_password(
        &req.account_id,
        &username,
        &req.new_password,
        req.engine,
    ) {
        Ok(user) => ok_response(json!({ "user": user })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn delete_database_user(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Path(username): Path<String>,
    Query(query): Query<DeleteDatabaseUserQuery>,
) -> Response {
    match state
        .databases
        .delete_user(&query.account_id, &username, query.engine)
    {
        Ok(user) => ok_response(json!({ "user": user })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn grant_database_access(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdateDatabaseGrantRequest>,
) -> Response {
    match state.databases.grant_access(req) {
        Ok(grant) => ok_response(json!({ "grant": grant })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn revoke_database_access(
    _claims: Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdateDatabaseGrantRequest>,
) -> Response {
    match state.databases.revoke_access(req) {
        Ok(grant) => ok_response(json!({ "grant": grant })).into_response(),
        Err(err) => db_error_response(err),
    }
}

async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match bearer_token(request.headers().get(AUTHORIZATION)) {
        Ok(token) => token,
        Err(response) => return response,
    };

    match decode_token(&state.config.jwt_secret, token) {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => {
            error_response(StatusCode::UNAUTHORIZED, "invalid or expired token").into_response()
        }
    }
}

async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.limiter.allow() {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate limit exceeded, retry later",
        )
        .into_response();
    }
    next.run(request).await
}

fn bearer_token(header: Option<&HeaderValue>) -> std::result::Result<&str, Response> {
    let value = header.and_then(|v| v.to_str().ok()).ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "missing Authorization header").into_response()
    })?;

    value.strip_prefix("Bearer ").ok_or_else(|| {
        error_response(StatusCode::UNAUTHORIZED, "invalid Authorization scheme").into_response()
    })
}

fn issue_token(config: &PanelConfig, username: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    let exp = now.saturating_add(config.jwt_ttl_seconds);
    let claims = Claims {
        sub: username.to_string(),
        role: "admin".to_string(),
        exp: exp as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .context("jwt encode failed")
}

fn decode_token(secret: &str, token: &str) -> Result<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|token_data| token_data.claims)
    .context("jwt decode failed")
}

fn cors_layer(config: &PanelConfig) -> Result<CorsLayer> {
    let base = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    if config
        .cors_allowed_origins
        .iter()
        .any(|origin| origin == "*")
    {
        return Ok(base.allow_origin(Any));
    }

    let origins = config
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .with_context(|| format!("invalid CORS origin value: {origin}"))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(base.allow_origin(origins))
}

fn ok_response<T: Serialize>(data: T) -> Json<ApiEnvelope<T>> {
    Json(ApiEnvelope {
        ok: true,
        data: Some(data),
        error: None,
    })
}

fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ApiEnvelope<Value>>) {
    (
        status,
        Json(ApiEnvelope {
            ok: false,
            data: None,
            error: Some(message.to_string()),
        }),
    )
}

fn db_error_response(error: ApiError) -> Response {
    error_response(error.status, &error.message).into_response()
}

fn validate_account_id(account_id: &str) -> DbResult<()> {
    validate_identifier("account_id", account_id, 3, 32, true)
}

fn validate_database_name(database: &str) -> DbResult<()> {
    validate_identifier("database", database, 3, 64, false)
}

fn validate_db_username(username: &str) -> DbResult<()> {
    validate_identifier("database username", username, 3, 64, false)
}

fn validate_identifier(
    field: &str,
    value: &str,
    min_len: usize,
    max_len: usize,
    allow_dash: bool,
) -> DbResult<()> {
    if value.len() < min_len || value.len() > max_len {
        return Err(ApiError::bad_request(format!(
            "{field} length must be between {min_len} and {max_len} characters"
        )));
    }

    let is_valid = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || (allow_dash && ch == '-'));

    if !is_valid {
        return Err(ApiError::bad_request(format!(
            "{field} must contain only letters, numbers, and '_'"
        )));
    }

    Ok(())
}

fn ensure_account_prefix(account_id: &str, value: &str, field: &str) -> DbResult<()> {
    let required_prefix = format!("{account_id}_");
    if !value.starts_with(&required_prefix) {
        return Err(ApiError::bad_request(format!(
            "{field} must start with '{required_prefix}'"
        )));
    }
    Ok(())
}

fn validate_password(password: &str) -> DbResult<()> {
    if password.len() < 8 {
        return Err(ApiError::bad_request(
            "password must be at least 8 characters long",
        ));
    }
    Ok(())
}

fn read_env(key: &str, default: &str) -> Result<String> {
    match env::var(key) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(err) => Err(anyhow!("failed to read {key}: {err}")),
    }
}

fn read_env_u64(key: &str, default: u64) -> Result<u64> {
    let value = read_env(key, &default.to_string())?;
    value
        .parse::<u64>()
        .with_context(|| format!("failed to parse {key} as u64"))
}

struct RateLimiter {
    max_requests_per_second: u64,
    inner: Mutex<RateWindow>,
}

struct RateWindow {
    started: Instant,
    count: u64,
}

impl RateLimiter {
    fn new(max_requests_per_second: u64) -> Self {
        Self {
            max_requests_per_second,
            inner: Mutex::new(RateWindow {
                started: Instant::now(),
                count: 0,
            }),
        }
    }

    fn allow(&self) -> bool {
        let mut window = self.inner.lock().expect("rate limiter mutex poisoned");
        if window.started.elapsed() >= Duration::from_secs(1) {
            window.started = Instant::now();
            window.count = 0;
        }

        if window.count >= self.max_requests_per_second {
            return false;
        }

        window.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request as HttpRequest};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_config() -> PanelConfig {
        PanelConfig {
            bind_addr: "127.0.0.1:7070".parse().unwrap(),
            jwt_secret: "test-secret".to_string(),
            jwt_ttl_seconds: 3600,
            admin_username: "admin".to_string(),
            admin_password: "password123".to_string(),
            cors_allowed_origins: vec!["*".to_string()],
            rate_limit_rps: 100,
        }
    }

    async fn read_json(response: Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn login_token(app: &Router) -> String {
        let login_request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"username":"admin","password":"password123"}"#,
            ))
            .unwrap();

        let login_response = app.clone().oneshot(login_request).await.unwrap();
        assert_eq!(login_response.status(), StatusCode::OK);
        let login_json = read_json(login_response).await;
        login_json["data"]["token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn login_and_auth_me_work() {
        let app = build_router(AppState::new(test_config())).unwrap();
        let token = login_token(&app).await;

        let me_request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/auth/me")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let me_response = app.oneshot(me_request).await.unwrap();
        assert_eq!(me_response.status(), StatusCode::OK);
        let me_json = read_json(me_response).await;
        assert_eq!(me_json["data"]["username"], "admin");
        assert_eq!(me_json["data"]["role"], "admin");
    }

    #[tokio::test]
    async fn auth_me_requires_token() {
        let app = build_router(AppState::new(test_config())).unwrap();
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/auth/me")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn global_rate_limit_applies() {
        let mut config = test_config();
        config.rate_limit_rps = 1;
        let app = build_router(AppState::new(config)).unwrap();

        let req1 = HttpRequest::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        let req2 = HttpRequest::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn database_module_crud_and_grants_work() {
        let app = build_router(AppState::new(test_config())).unwrap();
        let token = login_token(&app).await;

        let create_user = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/database-users")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","username":"acct1_app","password":"superpass1","engine":"mysql"}"#,
            ))
            .unwrap();
        let create_user_response = app.clone().oneshot(create_user).await.unwrap();
        assert_eq!(create_user_response.status(), StatusCode::OK);

        let create_db = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/databases")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","name":"acct1_main","engine":"mysql"}"#,
            ))
            .unwrap();
        let create_db_response = app.clone().oneshot(create_db).await.unwrap();
        assert_eq!(create_db_response.status(), StatusCode::OK);

        let grant = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/database-grants")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","username":"acct1_app","database":"acct1_main","engine":"mysql"}"#,
            ))
            .unwrap();
        let grant_response = app.clone().oneshot(grant).await.unwrap();
        assert_eq!(grant_response.status(), StatusCode::OK);

        let list_databases = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/databases?account_id=acct1&engine=mysql")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let list_databases_response = app.clone().oneshot(list_databases).await.unwrap();
        assert_eq!(list_databases_response.status(), StatusCode::OK);
        let list_databases_json = read_json(list_databases_response).await;
        assert_eq!(
            list_databases_json["data"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_databases_json["data"]["items"][0]["granted_users"]
                .as_u64()
                .unwrap(),
            1
        );

        let list_users = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/database-users?account_id=acct1&engine=mysql")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let list_users_response = app.clone().oneshot(list_users).await.unwrap();
        assert_eq!(list_users_response.status(), StatusCode::OK);
        let list_users_json = read_json(list_users_response).await;
        assert_eq!(
            list_users_json["data"]["items"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            list_users_json["data"]["items"][0]["granted_databases"]
                .as_u64()
                .unwrap(),
            1
        );

        let rotate_password = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/database-users/acct1_app/rotate-password")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","new_password":"superpass2","engine":"mysql"}"#,
            ))
            .unwrap();
        let rotate_response = app.clone().oneshot(rotate_password).await.unwrap();
        assert_eq!(rotate_response.status(), StatusCode::OK);
        let rotate_json = read_json(rotate_response).await;
        assert_eq!(
            rotate_json["data"]["user"]["password_version"]
                .as_u64()
                .unwrap(),
            2
        );

        let revoke = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/database-grants/revoke")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","username":"acct1_app","database":"acct1_main","engine":"mysql"}"#,
            ))
            .unwrap();
        let revoke_response = app.clone().oneshot(revoke).await.unwrap();
        assert_eq!(revoke_response.status(), StatusCode::OK);

        let delete_user = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/api/database-users/acct1_app?account_id=acct1&engine=mysql")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let delete_user_response = app.clone().oneshot(delete_user).await.unwrap();
        assert_eq!(delete_user_response.status(), StatusCode::OK);

        let delete_db = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/api/databases/acct1_main?account_id=acct1&engine=mysql")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let delete_db_response = app.oneshot(delete_db).await.unwrap();
        assert_eq!(delete_db_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cross_account_database_access_is_rejected() {
        let app = build_router(AppState::new(test_config())).unwrap();
        let token = login_token(&app).await;

        let create_db = HttpRequest::builder()
            .method(Method::POST)
            .uri("/api/databases")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"account_id":"acct1","name":"acct1_main","engine":"mysql"}"#,
            ))
            .unwrap();
        let create_db_response = app.clone().oneshot(create_db).await.unwrap();
        assert_eq!(create_db_response.status(), StatusCode::OK);

        let delete_as_other_account = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/api/databases/acct1_main?account_id=acct2&engine=mysql")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let delete_response = app.oneshot(delete_as_other_account).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::FORBIDDEN);
    }
}
