use crate::config::{CacheConfig, Config, VirtualHostConfig};

use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{BodyExt, Empty};
use hyper::{Method, Request};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct WarmTarget {
    pub domain: String,
    pub path: String,
    pub trigger: String,
    pub attempt: u8,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct WarmRequestPayload {
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub strategy: Option<String>,
}

#[derive(Default)]
struct WarmStats {
    queue_depth: AtomicU64,
    queued_total: AtomicU64,
    suppressed_total: AtomicU64,
    rejected_total: AtomicU64,
    processed_total: AtomicU64,
    success_total: AtomicU64,
    failure_total: AtomicU64,
    retried_total: AtomicU64,
    scheduled_runs: AtomicU64,
    manual_runs: AtomicU64,
    latency_total_ms: AtomicU64,
    latency_samples: AtomicU64,
    last_success_epoch: AtomicU64,
    last_failure_epoch: AtomicU64,
}

pub struct CacheWarmer {
    config: Arc<Config>,
    cache_config: CacheConfig,
    sender: mpsc::Sender<WarmTarget>,
    pending: DashMap<String, u64>,
    stats: WarmStats,
    started: AtomicBool,
}

impl CacheWarmer {
    pub fn new(config: Arc<Config>) -> Arc<Self> {
        let max_queue = config.cache.warm_max_queue_size.max(1);
        let (sender, receiver) = mpsc::channel(max_queue);

        let warmer = Arc::new(Self {
            cache_config: config.cache.clone(),
            config,
            sender,
            pending: DashMap::new(),
            stats: WarmStats::default(),
            started: AtomicBool::new(false),
        });

        warmer.clone().spawn_dispatcher(receiver);
        warmer
    }

    pub fn start(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::Relaxed) {
            return;
        }

        if !self.cache_config.warm_enabled {
            info!("cache warmer disabled via config");
            return;
        }

        let schedule_secs = self.cache_config.warm_schedule_secs;
        if schedule_secs > 0 {
            let warmer = self.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(schedule_secs));
                loop {
                    interval.tick().await;
                    if let Err(err) = warmer.enqueue_deterministic("scheduled").await {
                        warn!("scheduled cache warm enqueue failed: {}", err);
                    }
                }
            });
            info!("cache warmer schedule enabled every {}s", schedule_secs);
        }
    }

    fn spawn_dispatcher(self: Arc<Self>, mut receiver: mpsc::Receiver<WarmTarget>) {
        tokio::spawn(async move {
            let concurrency = self.cache_config.warm_max_concurrency.max(1);
            let limiter = Arc::new(Semaphore::new(concurrency));

            while let Some(target) = receiver.recv().await {
                self.stats.queue_depth.fetch_sub(1, Ordering::Relaxed);
                let permit = match limiter.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => break,
                };
                let warmer = self.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    warmer.process_target(target).await;
                });
            }
        });
    }

    async fn process_target(&self, target: WarmTarget) {
        self.stats.processed_total.fetch_add(1, Ordering::Relaxed);

        let key = format!("{}{}", target.domain, target.path);
        self.pending.remove(&key);

        let started = std::time::Instant::now();
        let outcome = self.warm_once(&target).await;
        let latency_ms = started.elapsed().as_millis() as u64;
        self.stats
            .latency_total_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.stats.latency_samples.fetch_add(1, Ordering::Relaxed);

        match outcome {
            Ok(_) => {
                self.stats.success_total.fetch_add(1, Ordering::Relaxed);
                self.stats
                    .last_success_epoch
                    .store(now_epoch_secs(), Ordering::Relaxed);
            }
            Err(err) => {
                if target.attempt < self.cache_config.warm_max_retries {
                    self.stats.retried_total.fetch_add(1, Ordering::Relaxed);
                    let backoff_ms = self.cache_config.warm_retry_backoff_ms
                        * (1u64 << target.attempt.min(6) as u64);
                    let sender = self.sender.clone();
                    let retry_target = WarmTarget {
                        attempt: target.attempt + 1,
                        ..target.clone()
                    };
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        let _ = sender.send(retry_target).await;
                    });
                    self.stats.queue_depth.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.stats.failure_total.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .last_failure_epoch
                        .store(now_epoch_secs(), Ordering::Relaxed);
                    warn!(
                        domain = %target.domain,
                        path = %target.path,
                        trigger = %target.trigger,
                        attempts = target.attempt + 1,
                        "cache warm target failed: {}",
                        err
                    );
                }
            }
        }
    }

    async fn warm_once(&self, target: &WarmTarget) -> anyhow::Result<()> {
        let origin = local_origin(&self.config.server.listen)?;
        let uri = format!("{}{}", origin, target.path);

        let connector = HttpConnector::new();
        let client: Client<_, Empty<Bytes>> =
            Client::builder(TokioExecutor::new()).build(connector);
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Host", &target.domain)
            .header("x-veloserve-cache-warm", "1")
            .body(Empty::new())?;

        let response = timeout(
            Duration::from_millis(self.cache_config.warm_request_timeout_ms),
            client.request(request),
        )
        .await
        .map_err(|_| anyhow::anyhow!("warm request timeout"))??;

        let status = response.status();
        let _ = response.into_body().collect().await;

        if status.is_success() || status.is_redirection() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "unexpected warm response status: {}",
                status
            ))
        }
    }

    pub async fn enqueue_deterministic(&self, trigger: &str) -> anyhow::Result<serde_json::Value> {
        let mut targets = Vec::new();
        for vhost in &self.config.virtualhost {
            targets.extend(self.deterministic_targets_for_vhost(vhost));
        }

        if targets.is_empty() {
            return Ok(json!({
                "queued": 0,
                "suppressed": 0,
                "rejected": 0,
                "targets": 0,
                "strategy": "deterministic"
            }));
        }

        let batch_size = self.cache_config.warm_batch_size.max(1);
        if targets.len() > batch_size {
            targets.truncate(batch_size);
        }

        let urls: Vec<String> = targets
            .into_iter()
            .map(|(domain, path)| format!("https://{}{}", domain, path))
            .collect();

        if trigger == "scheduled" {
            self.stats.scheduled_runs.fetch_add(1, Ordering::Relaxed);
        }

        self.enqueue_urls(&urls, None, trigger).await
    }

    pub async fn enqueue_from_payload(
        &self,
        payload: WarmRequestPayload,
    ) -> anyhow::Result<serde_json::Value> {
        let trigger = payload
            .trigger
            .as_deref()
            .unwrap_or("manual")
            .to_ascii_lowercase();

        if trigger == "manual" {
            self.stats.manual_runs.fetch_add(1, Ordering::Relaxed);
        }

        let strategy = payload
            .strategy
            .as_deref()
            .unwrap_or(if payload.urls.is_empty() {
                "deterministic"
            } else {
                "urls"
            })
            .to_ascii_lowercase();

        if strategy == "deterministic" {
            return self.enqueue_deterministic(&trigger).await;
        }

        self.enqueue_urls(&payload.urls, payload.domain.as_deref(), &trigger)
            .await
    }

    async fn enqueue_urls(
        &self,
        urls: &[String],
        fallback_domain: Option<&str>,
        trigger: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let default_domain = self.default_domain();
        let fallback_domain = fallback_domain.or(default_domain.as_deref());
        let mut queued = 0u64;
        let mut suppressed = 0u64;
        let mut rejected = 0u64;

        for raw in urls {
            let Some((domain, path)) = normalize_target(raw, fallback_domain) else {
                rejected += 1;
                continue;
            };

            let key = format!("{}{}", domain, path);
            let now = now_epoch_secs();
            self.pending.retain(|_, ts| {
                now.saturating_sub(*ts) <= self.cache_config.warm_dedupe_window_secs
            });

            if self
                .pending
                .get(&key)
                .map(|ts| now.saturating_sub(*ts) <= self.cache_config.warm_dedupe_window_secs)
                .unwrap_or(false)
            {
                suppressed += 1;
                continue;
            }

            let target = WarmTarget {
                domain,
                path,
                trigger: trigger.to_string(),
                attempt: 0,
            };

            match self.sender.try_send(target) {
                Ok(_) => {
                    self.pending.insert(key, now);
                    queued += 1;
                    self.stats.queue_depth.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    rejected += 1;
                }
            }
        }

        self.stats.queued_total.fetch_add(queued, Ordering::Relaxed);
        self.stats
            .suppressed_total
            .fetch_add(suppressed, Ordering::Relaxed);
        self.stats
            .rejected_total
            .fetch_add(rejected, Ordering::Relaxed);

        Ok(json!({
            "queued": queued,
            "suppressed": suppressed,
            "rejected": rejected,
            "strategy": "urls",
            "queue_depth": self.stats.queue_depth.load(Ordering::Relaxed)
        }))
    }

    fn deterministic_targets_for_vhost(&self, vhost: &VirtualHostConfig) -> Vec<(String, String)> {
        let mut paths = HashSet::new();
        paths.insert("/".to_string());

        match vhost.platform.as_deref().map(|p| p.to_ascii_lowercase()) {
            Some(platform) if platform.contains("wordpress") => {
                paths.insert("/blog/".to_string());
                paths.insert("/category/".to_string());
                paths.insert("/tag/".to_string());
                paths.insert("/shop/".to_string());
                paths.insert("/product/".to_string());
            }
            Some(platform) if platform.contains("magento") => {
                paths.insert("/catalog/".to_string());
                paths.insert("/catalog/category/".to_string());
                paths.insert("/catalog/product/".to_string());
                paths.insert("/search/".to_string());
            }
            _ => {}
        }

        for path in discover_key_landing_paths(&vhost.root) {
            paths.insert(path);
        }

        let mut out: Vec<(String, String)> = paths
            .into_iter()
            .map(|path| (vhost.domain.clone(), path))
            .collect();
        out.sort();
        out
    }

    fn default_domain(&self) -> Option<String> {
        self.config
            .virtualhost
            .iter()
            .find(|v| v.domain != "*")
            .map(|v| v.domain.clone())
            .or_else(|| self.config.virtualhost.first().map(|v| v.domain.clone()))
    }

    pub fn stats_json(&self) -> serde_json::Value {
        let samples = self.stats.latency_samples.load(Ordering::Relaxed);
        let avg_latency_ms = if samples == 0 {
            0
        } else {
            self.stats.latency_total_ms.load(Ordering::Relaxed) / samples
        };

        json!({
            "enabled": self.cache_config.warm_enabled,
            "queue_depth": self.stats.queue_depth.load(Ordering::Relaxed),
            "queued_total": self.stats.queued_total.load(Ordering::Relaxed),
            "suppressed_total": self.stats.suppressed_total.load(Ordering::Relaxed),
            "rejected_total": self.stats.rejected_total.load(Ordering::Relaxed),
            "processed_total": self.stats.processed_total.load(Ordering::Relaxed),
            "success_total": self.stats.success_total.load(Ordering::Relaxed),
            "failure_total": self.stats.failure_total.load(Ordering::Relaxed),
            "retried_total": self.stats.retried_total.load(Ordering::Relaxed),
            "scheduled_runs": self.stats.scheduled_runs.load(Ordering::Relaxed),
            "manual_runs": self.stats.manual_runs.load(Ordering::Relaxed),
            "avg_latency_ms": avg_latency_ms,
            "last_success_epoch": self.stats.last_success_epoch.load(Ordering::Relaxed),
            "last_failure_epoch": self.stats.last_failure_epoch.load(Ordering::Relaxed),
            "config": {
                "schedule_secs": self.cache_config.warm_schedule_secs,
                "max_queue_size": self.cache_config.warm_max_queue_size,
                "max_concurrency": self.cache_config.warm_max_concurrency,
                "request_timeout_ms": self.cache_config.warm_request_timeout_ms,
                "max_retries": self.cache_config.warm_max_retries,
                "retry_backoff_ms": self.cache_config.warm_retry_backoff_ms,
                "dedupe_window_secs": self.cache_config.warm_dedupe_window_secs,
                "batch_size": self.cache_config.warm_batch_size,
            }
        })
    }
}

fn local_origin(listen: &str) -> anyhow::Result<String> {
    let addr: SocketAddr = listen.parse()?;
    let host = if addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else {
        addr.ip().to_string()
    };
    Ok(format!("http://{}:{}", host, addr.port()))
}

fn discover_key_landing_paths(root: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let deny_dirs = [
        "wp-admin",
        "wp-content",
        "vendor",
        "node_modules",
        "var",
        "cache",
        ".git",
    ];

    let root_path = Path::new(root);
    if let Ok(entries) = std::fs::read_dir(root_path) {
        for entry in entries.flatten().take(32) {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            if path.is_dir() {
                if deny_dirs.contains(&name) || name.starts_with('.') {
                    continue;
                }
                paths.push(format!("/{}/", name));
            } else if path.is_file() {
                if name == "index.html" || name == "index.php" {
                    continue;
                }
                if name.ends_with(".html") || name.ends_with(".htm") || name.ends_with(".php") {
                    paths.push(format!("/{}", name));
                }
            }
        }
    }

    paths
}

fn normalize_target(raw: &str, fallback_domain: Option<&str>) -> Option<(String, String)> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if value.starts_with("http://") || value.starts_with("https://") {
        let (scheme_split, rest) = value.split_once("://")?;
        if scheme_split.is_empty() {
            return None;
        }
        let (host, path_part) = if let Some((host, path)) = rest.split_once('/') {
            (host, format!("/{}", path))
        } else {
            (rest, "/".to_string())
        };
        let domain = host.split(':').next()?.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return None;
        }
        let path = sanitize_path(&path_part)?;
        return Some((domain, path));
    }

    if value.starts_with('/') {
        let domain = fallback_domain?.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return None;
        }
        let path = sanitize_path(value)?;
        return Some((domain, path));
    }

    let domain = fallback_domain?.trim().to_ascii_lowercase();
    if domain.is_empty() {
        return None;
    }
    let path = sanitize_path(&format!("/{}", value))?;
    Some((domain, path))
}

fn sanitize_path(path: &str) -> Option<String> {
    let without_query = path
        .split('?')
        .next()
        .unwrap_or("/")
        .split('#')
        .next()
        .unwrap_or("/")
        .trim();

    if without_query.is_empty() {
        return Some("/".to_string());
    }

    let mut normalized = if without_query.starts_with('/') {
        without_query.to_string()
    } else {
        format!("/{}", without_query)
    };

    if !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }

    if normalized.contains("..") {
        return None;
    }

    Some(normalized)
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}
