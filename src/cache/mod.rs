//! Cache Module
//!
//! Multi-layer caching system for VeloServe.

use crate::config::{CacheConfig, CacheStorage};
use dashmap::DashMap;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use lru::LruCache;
use parking_lot::Mutex;
use redis::{Client, Commands, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    content_type: String,
    tags: Vec<String>,
    created_at_epoch_secs: u64,
    ttl: Duration,
    stale_after: Duration,
}

impl CacheEntry {
    fn new(
        data: Vec<u8>,
        content_type: String,
        tags: Vec<String>,
        ttl: Duration,
        stale_after: Duration,
    ) -> Self {
        Self {
            data,
            content_type,
            tags,
            created_at_epoch_secs: now_epoch_secs(),
            ttl,
            stale_after,
        }
    }

    fn from_persisted(persisted: PersistedEntry) -> Self {
        Self {
            data: persisted.data,
            content_type: persisted.content_type,
            tags: persisted.tags,
            created_at_epoch_secs: persisted.created_at_epoch_secs,
            ttl: Duration::from_secs(persisted.ttl_seconds),
            stale_after: Duration::from_secs(persisted.stale_after_seconds),
        }
    }

    fn to_persisted(&self) -> PersistedEntry {
        PersistedEntry {
            key: String::new(),
            data: self.data.clone(),
            content_type: self.content_type.clone(),
            tags: self.tags.clone(),
            created_at_epoch_secs: self.created_at_epoch_secs,
            ttl_seconds: self.ttl.as_secs(),
            stale_after_seconds: self.stale_after.as_secs(),
        }
    }

    fn age_seconds(&self) -> u64 {
        now_epoch_secs().saturating_sub(self.created_at_epoch_secs)
    }

    fn is_expired(&self) -> bool {
        self.age_seconds() > self.ttl.as_secs()
    }

    fn is_stale(&self) -> bool {
        self.age_seconds() > self.stale_after.as_secs()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CacheLifetime {
    pub ttl: Duration,
    pub stale_after: Duration,
}

impl CacheLifetime {
    pub fn new(ttl: Duration, stale_after: Duration) -> Self {
        let stale_after = stale_after.min(ttl);
        Self { ttl, stale_after }
    }

    pub fn from_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            stale_after: ttl,
        }
    }
}

#[derive(Default)]
struct LayerStats {
    hits: AtomicU64,
    misses: AtomicU64,
    writes: AtomicU64,
    evictions: AtomicU64,
    stale: AtomicU64,
    errors: AtomicU64,
    fallbacks: AtomicU64,
    ops: AtomicU64,
    op_latency_micros: AtomicU64,
}

#[derive(Default)]
struct CacheStats {
    l1: LayerStats,
    l2: LayerStats,
    size_bytes: AtomicU64,
}

const REDIS_ENTRY_VERSION: u8 = 1;
const REDIS_COMPRESSION_THRESHOLD_BYTES: usize = 1024;
const REDIS_RETRY_ATTEMPTS: u32 = 2;
const REDIS_TAG_INDEX_TTL_GRACE_SECS: u64 = 300;

#[derive(Serialize, Deserialize)]
struct RedisPersistedEntry {
    version: u8,
    content_type: String,
    tags: Vec<String>,
    created_at_epoch_secs: u64,
    ttl_seconds: u64,
    stale_after_seconds: u64,
    compressed: bool,
    data: Vec<u8>,
}

trait PersistentCacheLayer: Send + Sync {
    fn get(&self, key: &str) -> Option<CacheEntry>;
    fn set(&self, key: &str, entry: &CacheEntry) -> std::io::Result<()>;
    fn remove(&self, key: &str) -> std::io::Result<()>;
    fn purge_by_tag(&self, tag: &str) -> std::io::Result<usize>;
    fn purge_by_prefix(&self, prefix: &str) -> std::io::Result<usize>;
    fn purge_all(&self) -> std::io::Result<usize>;
}

#[derive(Serialize, Deserialize)]
struct PersistedEntry {
    #[serde(default)]
    key: String,
    data: Vec<u8>,
    content_type: String,
    tags: Vec<String>,
    created_at_epoch_secs: u64,
    ttl_seconds: u64,
    stale_after_seconds: u64,
}

struct DiskCacheLayer {
    root: PathBuf,
    io_lock: Mutex<()>,
}

impl DiskCacheLayer {
    fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            io_lock: Mutex::new(()),
        })
    }

    fn key_path(&self, key: &str) -> PathBuf {
        self.root
            .join(format!("{}.bin", filesystem_safe_key(key.as_bytes())))
    }

    fn entry_paths(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
        Ok(files)
    }

    fn read_entry(&self, path: &Path) -> Option<PersistedEntry> {
        let bytes = fs::read(path).ok()?;
        bincode::deserialize::<PersistedEntry>(&bytes).ok()
    }

    fn write_entry(&self, path: &Path, entry: &PersistedEntry) -> std::io::Result<()> {
        let bytes = bincode::serialize(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(path, bytes)
    }
}

impl PersistentCacheLayer for DiskCacheLayer {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        let _guard = self.io_lock.lock();
        let path = self.key_path(key);
        let persisted = self.read_entry(&path)?;
        Some(CacheEntry::from_persisted(persisted))
    }

    fn set(&self, key: &str, entry: &CacheEntry) -> std::io::Result<()> {
        let _guard = self.io_lock.lock();
        let path = self.key_path(key);
        let mut persisted = entry.to_persisted();
        persisted.key = key.to_string();
        self.write_entry(&path, &persisted)
    }

    fn remove(&self, key: &str) -> std::io::Result<()> {
        let _guard = self.io_lock.lock();
        let path = self.key_path(key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn purge_by_tag(&self, tag: &str) -> std::io::Result<usize> {
        let _guard = self.io_lock.lock();
        let mut removed = 0;
        for path in self.entry_paths()? {
            if let Some(entry) = self.read_entry(&path) {
                if entry.tags.iter().any(|current| current == tag) {
                    fs::remove_file(path)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    fn purge_by_prefix(&self, prefix: &str) -> std::io::Result<usize> {
        let _guard = self.io_lock.lock();
        let mut removed = 0;
        for path in self.entry_paths()? {
            if let Some(entry) = self.read_entry(&path) {
                if entry.key.starts_with(prefix) {
                    fs::remove_file(path)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    fn purge_all(&self) -> std::io::Result<usize> {
        let _guard = self.io_lock.lock();
        let mut removed = 0;
        for path in self.entry_paths()? {
            fs::remove_file(path)?;
            removed += 1;
        }
        Ok(removed)
    }
}

struct RedisCacheLayer {
    client: Client,
    pool: Mutex<Vec<Connection>>,
    max_pool_size: usize,
    namespace: String,
}

impl RedisCacheLayer {
    fn new(redis_url: &str) -> std::io::Result<Self> {
        let client = Client::open(redis_url).map_err(to_io_error)?;

        Ok(Self {
            client,
            pool: Mutex::new(Vec::new()),
            max_pool_size: 8,
            namespace: "veloserve:v1".to_string(),
        })
    }

    fn entry_key(&self, key: &str) -> String {
        format!("{}:entry:{}", self.namespace, key)
    }

    fn tag_key(&self, tag: &str) -> String {
        format!("{}:tag:{}", self.namespace, normalize_cache_key(tag))
    }

    fn key_index_key(&self) -> String {
        format!("{}:keys", self.namespace)
    }

    fn acquire_connection(&self) -> std::io::Result<Connection> {
        if let Some(conn) = self.pool.lock().pop() {
            return Ok(conn);
        }
        self.client
            .get_connection()
            .map_err(|err| to_io_error(format!("redis connection failed: {}", err)))
    }

    fn release_connection(&self, conn: Connection) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            pool.push(conn);
        }
    }

    fn with_conn<T, F>(&self, mut op: F) -> std::io::Result<T>
    where
        F: FnMut(&mut Connection) -> redis::RedisResult<T>,
    {
        for attempt in 0..=REDIS_RETRY_ATTEMPTS {
            let mut conn = self.acquire_connection()?;
            match op(&mut conn) {
                Ok(value) => {
                    self.release_connection(conn);
                    return Ok(value);
                }
                Err(err) => {
                    drop(conn);
                    if attempt == REDIS_RETRY_ATTEMPTS {
                        return Err(to_io_error(err));
                    }
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "redis retry budget exhausted",
        ))
    }

    fn serialize_entry(entry: &CacheEntry) -> std::io::Result<Vec<u8>> {
        let (compressed, data) = if entry.data.len() >= REDIS_COMPRESSION_THRESHOLD_BYTES {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&entry.data)?;
            let compressed = encoder.finish()?;
            if compressed.len() < entry.data.len() {
                (true, compressed)
            } else {
                (false, entry.data.clone())
            }
        } else {
            (false, entry.data.clone())
        };

        let persisted = RedisPersistedEntry {
            version: REDIS_ENTRY_VERSION,
            content_type: entry.content_type.clone(),
            tags: entry.tags.clone(),
            created_at_epoch_secs: entry.created_at_epoch_secs,
            ttl_seconds: entry.ttl.as_secs(),
            stale_after_seconds: entry.stale_after.as_secs(),
            compressed,
            data,
        };

        bincode::serialize(&persisted)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))
    }

    fn deserialize_entry(raw: &[u8]) -> Option<CacheEntry> {
        let persisted: RedisPersistedEntry = bincode::deserialize(raw).ok()?;
        if persisted.version != REDIS_ENTRY_VERSION {
            return None;
        }

        let data = if persisted.compressed {
            let mut decoder = GzDecoder::new(persisted.data.as_slice());
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).ok()?;
            out
        } else {
            persisted.data
        };

        Some(CacheEntry {
            data,
            content_type: persisted.content_type,
            tags: persisted.tags,
            created_at_epoch_secs: persisted.created_at_epoch_secs,
            ttl: Duration::from_secs(persisted.ttl_seconds),
            stale_after: Duration::from_secs(persisted.stale_after_seconds),
        })
    }

    fn remove_internal(&self, conn: &mut Connection, key: &str) -> redis::RedisResult<bool> {
        let entry_key = self.entry_key(key);
        let key_index_key = self.key_index_key();

        let raw: Option<Vec<u8>> = conn.get(&entry_key)?;
        let mut removed = false;

        if let Some(raw) = raw {
            if let Some(entry) = Self::deserialize_entry(&raw) {
                for tag in entry.tags {
                    let _: usize = conn.srem(self.tag_key(&tag), key)?;
                }
            }
            let deleted: usize = conn.del(&entry_key)?;
            removed = deleted > 0;
        }

        let _: usize = conn.srem(key_index_key, key)?;
        Ok(removed)
    }
}

impl PersistentCacheLayer for RedisCacheLayer {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        let entry_key = self.entry_key(key);
        let raw = self
            .with_conn(|conn| conn.get::<_, Option<Vec<u8>>>(&entry_key))
            .ok()?;
        raw.and_then(|bytes| Self::deserialize_entry(&bytes))
    }

    fn set(&self, key: &str, entry: &CacheEntry) -> std::io::Result<()> {
        let entry_key = self.entry_key(key);
        let key_index_key = self.key_index_key();
        let payload = Self::serialize_entry(entry)?;
        let ttl_secs = entry.ttl.as_secs().max(1);
        let tag_ttl = ttl_secs.saturating_add(REDIS_TAG_INDEX_TTL_GRACE_SECS);

        self.with_conn(|conn| {
            let _: () = conn.set_ex(&entry_key, payload.clone(), ttl_secs)?;
            let _: usize = conn.sadd(&key_index_key, key)?;
            for tag in &entry.tags {
                let tag_key = self.tag_key(tag);
                let _: usize = conn.sadd(&tag_key, key)?;
                let _: bool = conn.expire(&tag_key, tag_ttl as i64)?;
            }
            Ok(())
        })
    }

    fn remove(&self, key: &str) -> std::io::Result<()> {
        self.with_conn(|conn| self.remove_internal(conn, key).map(|_| ()))
    }

    fn purge_by_tag(&self, tag: &str) -> std::io::Result<usize> {
        let tag_key = self.tag_key(tag);
        self.with_conn(|conn| {
            let keys: Vec<String> = conn.smembers(&tag_key)?;
            let mut removed = 0usize;
            for key in &keys {
                if self.remove_internal(conn, key)? {
                    removed += 1;
                }
            }
            let _: usize = conn.del(&tag_key)?;
            Ok(removed)
        })
    }

    fn purge_by_prefix(&self, prefix: &str) -> std::io::Result<usize> {
        let key_index_key = self.key_index_key();
        self.with_conn(|conn| {
            let keys: Vec<String> = conn.smembers(&key_index_key)?;
            let mut removed = 0usize;
            for key in keys {
                if key.starts_with(prefix) && self.remove_internal(conn, &key)? {
                    removed += 1;
                }
            }
            Ok(removed)
        })
    }

    fn purge_all(&self) -> std::io::Result<usize> {
        let key_index_key = self.key_index_key();
        self.with_conn(|conn| {
            let keys: Vec<String> = conn.smembers(&key_index_key)?;
            let mut removed = 0usize;
            for key in keys {
                if self.remove_internal(conn, &key)? {
                    removed += 1;
                }
            }
            let _: usize = conn.del(&key_index_key)?;
            Ok(removed)
        })
    }
}

/// Cache manager
pub struct CacheManager {
    l1_cache: DashMap<String, CacheEntry>,
    l1_lru: Mutex<LruCache<String, ()>>,
    tag_index: DashMap<String, Vec<String>>,
    config: CacheConfig,
    stats: CacheStats,
    max_memory: u64,
    l2_cache: Option<Box<dyn PersistentCacheLayer>>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: &CacheConfig) -> Self {
        let max_memory = parse_size(&config.memory_limit);
        let max_entries = NonZeroUsize::new(10_000).expect("non-zero LRU size");

        let l2_cache = if config.l2_enabled {
            match config.storage {
                CacheStorage::Redis => {
                    if let Some(redis_url) = config.redis_url.as_deref() {
                        match RedisCacheLayer::new(redis_url) {
                            Ok(layer) => Some(Box::new(layer) as Box<dyn PersistentCacheLayer>),
                            Err(err) => {
                                warn!("Failed to initialize Redis cache layer: {}", err);
                                None
                            }
                        }
                    } else {
                        warn!("Redis cache storage selected but cache.redis_url is not set; disabling L2");
                        None
                    }
                }
                CacheStorage::Memory | CacheStorage::Disk => {
                    match DiskCacheLayer::new(&config.disk_path) {
                        Ok(layer) => Some(Box::new(layer) as Box<dyn PersistentCacheLayer>),
                        Err(err) => {
                            warn!(
                                "Failed to initialize L2 disk cache at {}: {}",
                                config.disk_path, err
                            );
                            None
                        }
                    }
                }
            }
        } else {
            None
        };

        info!(
            "Initializing cache: l1_enabled={}, l2_enabled={}, storage={:?}, max_memory={}",
            config.l1_enabled,
            l2_cache.is_some(),
            config.storage,
            config.memory_limit
        );

        Self {
            l1_cache: DashMap::new(),
            l1_lru: Mutex::new(LruCache::new(max_entries)),
            tag_index: DashMap::new(),
            config: config.clone(),
            stats: CacheStats::default(),
            max_memory,
            l2_cache,
        }
    }

    /// Get an entry from cache
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_with_metadata(key).await.map(|(data, _)| data)
    }

    /// Get an entry and its content-type from cache
    pub async fn get_with_metadata(&self, key: &str) -> Option<(Vec<u8>, String)> {
        if !self.config.enable {
            return None;
        }

        let key = normalize_cache_key(key);

        if self.config.l1_enabled {
            if let Some(entry) = self.l1_cache.get(&key) {
                if entry.is_expired() {
                    drop(entry);
                    self.remove_l1(&key).await;
                    self.stats.l1.misses.fetch_add(1, Ordering::Relaxed);
                } else if entry.is_stale() {
                    drop(entry);
                    self.remove_l1(&key).await;
                    self.stats.l1.stale.fetch_add(1, Ordering::Relaxed);
                    self.stats.l1.misses.fetch_add(1, Ordering::Relaxed);
                } else {
                    {
                        let mut lru = self.l1_lru.lock();
                        lru.get(&key);
                    }
                    self.stats.l1.hits.fetch_add(1, Ordering::Relaxed);
                    debug!("L1 cache hit: {}", key);
                    return Some((entry.data.clone(), entry.content_type.clone()));
                }
            } else {
                self.stats.l1.misses.fetch_add(1, Ordering::Relaxed);
            }
        }

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            if let Some(entry) = l2.get(&key) {
                self.record_l2_op(started, true);
                if entry.is_expired() {
                    let _ = l2.remove(&key);
                    self.stats.l2.misses.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                if entry.is_stale() {
                    let _ = l2.remove(&key);
                    self.stats.l2.stale.fetch_add(1, Ordering::Relaxed);
                    self.stats.l2.misses.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                self.stats.l2.hits.fetch_add(1, Ordering::Relaxed);
                debug!("L2 cache hit: {}", key);

                if self.config.l1_enabled {
                    self.write_l1(&key, entry.clone()).await;
                }

                return Some((entry.data, entry.content_type));
            }
            self.record_l2_op(started, true);
            self.stats.l2.misses.fetch_add(1, Ordering::Relaxed);
        }

        None
    }

    /// Store an entry in cache using default layer policy.
    pub async fn set(&self, key: &str, data: Vec<u8>, content_type: &str, tags: Vec<String>) {
        if !self.config.enable {
            return;
        }

        let ttl = Duration::from_secs(self.config.default_ttl);
        self.set_with_ttl(key, data, content_type, tags, ttl).await;
    }

    /// Store an entry with custom ttl and default stale policy.
    pub async fn set_with_ttl(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
        tags: Vec<String>,
        ttl: Duration,
    ) {
        self.set_with_lifetime(key, data, content_type, tags, CacheLifetime::from_ttl(ttl))
            .await;
    }

    /// Store an entry with explicit ttl/stale policy.
    pub async fn set_with_lifetime(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
        tags: Vec<String>,
        lifetime: CacheLifetime,
    ) {
        if !self.config.enable {
            return;
        }

        let key = normalize_cache_key(key);
        let entry = CacheEntry::new(
            data,
            content_type.to_string(),
            tags.clone(),
            lifetime.ttl,
            lifetime.stale_after,
        );

        if self.config.l1_enabled {
            self.write_l1(&key, entry.clone()).await;
        }

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            if let Err(err) = l2.set(&key, &entry) {
                self.record_l2_op(started, false);
                warn!("Failed to write L2 cache key {}: {}", key, err);
            } else {
                self.record_l2_op(started, true);
                self.stats.l2.writes.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.index_tags(&key, &tags);
        debug!(
            "Cache set: {} ({} bytes, ttl={:?}, stale_after={:?})",
            key,
            entry.data.len(),
            entry.ttl,
            entry.stale_after
        );
    }

    /// Remove an entry from all cache layers.
    pub async fn remove(&self, key: &str) {
        let _ = self.remove_with_count(key).await;
    }

    /// Remove an entry from all cache layers and return affected entry count.
    pub async fn remove_with_count(&self, key: &str) -> usize {
        let key = normalize_cache_key(key);
        let mut affected = 0usize;
        if self.remove_l1(&key).await {
            affected += 1;
        }

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            let existed_in_l2 = l2.get(&key).is_some();
            if let Err(err) = l2.remove(&key) {
                self.record_l2_op(started, false);
                warn!("Failed to remove L2 cache key {}: {}", key, err);
            } else if existed_in_l2 {
                self.record_l2_op(started, true);
                affected += 1;
            } else {
                self.record_l2_op(started, true);
            }
        }

        affected
    }

    /// Purge all entries with a specific tag
    pub async fn purge_by_tag(&self, tag: &str) {
        let _ = self.purge_by_tag_count(tag).await;
    }

    /// Purge all entries with a specific tag and return affected entry count.
    pub async fn purge_by_tag_count(&self, tag: &str) -> usize {
        info!("Purging cache entries with tag: {}", tag);
        let mut affected = 0usize;

        if let Some((_, keys)) = self.tag_index.remove(tag) {
            for key in keys {
                if self.remove_l1(&key).await {
                    affected += 1;
                }
            }
        }

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            match l2.purge_by_tag(tag) {
                Ok(removed) => {
                    self.record_l2_op(started, true);
                    affected += removed;
                }
                Err(err) => {
                    self.record_l2_op(started, false);
                    warn!("Failed to purge L2 tag {}: {}", tag, err);
                }
            }
        }

        affected
    }

    /// Purge all entries whose key starts with a prefix.
    pub async fn purge_by_prefix(&self, prefix: &str) {
        let _ = self.purge_by_prefix_count(prefix).await;
    }

    /// Purge all entries whose key starts with a prefix and return affected entry count.
    pub async fn purge_by_prefix_count(&self, prefix: &str) -> usize {
        let prefix = normalize_cache_key(prefix);
        let mut affected = 0usize;
        let keys: Vec<String> = self
            .l1_cache
            .iter()
            .filter(|entry| entry.key().starts_with(&prefix))
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            if self.remove_l1(&key).await {
                affected += 1;
            }
        }

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            match l2.purge_by_prefix(&prefix) {
                Ok(removed) => {
                    self.record_l2_op(started, true);
                    affected += removed;
                }
                Err(err) => {
                    self.record_l2_op(started, false);
                    warn!("Failed to purge L2 key prefix {}: {}", prefix, err);
                }
            }
        }

        affected
    }

    /// Purge all cache entries.
    pub async fn purge_all(&self) {
        info!("Purging all cache entries");

        self.l1_cache.clear();
        self.tag_index.clear();

        {
            let mut lru = self.l1_lru.lock();
            lru.clear();
        }

        self.stats.size_bytes.store(0, Ordering::Relaxed);
        self.stats.l1.evictions.fetch_add(1, Ordering::Relaxed);

        if let Some(l2) = &self.l2_cache {
            let started = Instant::now();
            match l2.purge_all() {
                Ok(removed) => {
                    self.record_l2_op(started, true);
                    if removed > 0 {
                        self.stats
                            .l2
                            .evictions
                            .fetch_add(removed as u64, Ordering::Relaxed);
                    }
                }
                Err(err) => {
                    self.record_l2_op(started, false);
                    warn!("Failed to purge all L2 entries: {}", err)
                }
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> serde_json::Value {
        let l1_hits = self.stats.l1.hits.load(Ordering::Relaxed);
        let l1_misses = self.stats.l1.misses.load(Ordering::Relaxed);
        let l2_hits = self.stats.l2.hits.load(Ordering::Relaxed);
        let l2_misses = self.stats.l2.misses.load(Ordering::Relaxed);

        json!({
            "enabled": self.config.enable,
            "entries": self.l1_cache.len(),
            "size_bytes": self.stats.size_bytes.load(Ordering::Relaxed),
            "max_memory": self.max_memory,
            "l1": {
                "enabled": self.config.l1_enabled,
                "hits": l1_hits,
                "misses": l1_misses,
                "writes": self.stats.l1.writes.load(Ordering::Relaxed),
                "evictions": self.stats.l1.evictions.load(Ordering::Relaxed),
                "stale": self.stats.l1.stale.load(Ordering::Relaxed)
            },
            "l2": {
                "enabled": self.l2_cache.is_some(),
                "hits": l2_hits,
                "misses": l2_misses,
                "writes": self.stats.l2.writes.load(Ordering::Relaxed),
                "evictions": self.stats.l2.evictions.load(Ordering::Relaxed),
                "stale": self.stats.l2.stale.load(Ordering::Relaxed),
                "errors": self.stats.l2.errors.load(Ordering::Relaxed),
                "fallbacks": self.stats.l2.fallbacks.load(Ordering::Relaxed),
                "ops": self.stats.l2.ops.load(Ordering::Relaxed),
                "latency_ms_avg": avg_latency_ms(
                    self.stats.l2.op_latency_micros.load(Ordering::Relaxed),
                    self.stats.l2.ops.load(Ordering::Relaxed)
                )
            },
            "hit_rate": hit_rate(l1_hits + l2_hits, l1_misses + l2_misses),
        })
    }

    fn record_l2_op(&self, started: Instant, ok: bool) {
        self.stats.l2.ops.fetch_add(1, Ordering::Relaxed);
        self.stats
            .l2
            .op_latency_micros
            .fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
        if !ok {
            self.stats.l2.errors.fetch_add(1, Ordering::Relaxed);
            self.stats.l2.fallbacks.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn remove_l1(&self, key: &str) -> bool {
        let mut removed = false;
        if let Some((_, entry)) = self.l1_cache.remove(key) {
            removed = true;
            self.stats
                .size_bytes
                .fetch_sub(entry.data.len() as u64, Ordering::Relaxed);

            for tag in &entry.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.retain(|current| current != key);
                }
            }
        }

        {
            let mut lru = self.l1_lru.lock();
            lru.pop(key);
        }

        removed
    }

    async fn write_l1(&self, key: &str, entry: CacheEntry) {
        let entry_size = entry.data.len() as u64;
        if let Some(previous) = self.l1_cache.get(key) {
            self.stats
                .size_bytes
                .fetch_sub(previous.data.len() as u64, Ordering::Relaxed);
        }
        if self.stats.size_bytes.load(Ordering::Relaxed) + entry_size > self.max_memory {
            self.evict_lru().await;
        }

        self.l1_cache.insert(key.to_string(), entry);

        {
            let mut lru = self.l1_lru.lock();
            lru.put(key.to_string(), ());
        }

        self.stats
            .size_bytes
            .fetch_add(entry_size, Ordering::Relaxed);
        self.stats.l1.writes.fetch_add(1, Ordering::Relaxed);
    }

    fn index_tags(&self, key: &str, tags: &[String]) {
        for tag in tags {
            self.tag_index
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(key.to_string());
        }
    }

    async fn evict_lru(&self) {
        let mut evicted = 0;
        let target = self.max_memory * 8 / 10;

        while self.stats.size_bytes.load(Ordering::Relaxed) > target {
            let key_to_evict = {
                let mut lru = self.l1_lru.lock();
                lru.pop_lru().map(|(k, _)| k)
            };

            if let Some(key) = key_to_evict {
                self.remove_l1(&key).await;
                evicted += 1;
            } else {
                break;
            }
        }

        if evicted > 0 {
            debug!("Evicted {} L1 cache entries", evicted);
            self.stats
                .l1
                .evictions
                .fetch_add(evicted, Ordering::Relaxed);
        }
    }
}

/// Normalize cache key to a deterministic file-safe representation.
pub fn normalize_cache_key(raw: &str) -> String {
    let raw = raw.trim();
    let mut key = String::with_capacity(raw.len());

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '/' | '_' | '-' | '.') {
            key.push(ch);
        } else {
            key.push('_');
        }
    }

    key
}

fn filesystem_safe_key(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push_str(&format!("{:02x}", byte));
        }
    }
    out
}

/// Build deterministic cache key for page responses.
pub fn build_page_cache_key(host: &str, path_and_query: &str) -> String {
    let normalized_host = host
        .trim()
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_ascii_lowercase();

    let path = percent_encoding::percent_decode_str(path_and_query)
        .decode_utf8_lossy()
        .to_string();
    let path = normalize_path(&path);

    normalize_cache_key(&format!("page:{}:{}", normalized_host, path))
}

/// Build scoped cache key that avoids collisions across app/site/store/variant dimensions.
pub fn build_page_cache_key_scoped(
    host: &str,
    site: Option<&str>,
    store: Option<&str>,
    variant: Option<&str>,
    path_and_query: &str,
) -> String {
    let base = build_page_cache_key(host, path_and_query);
    let site = normalize_cache_key_part(site.unwrap_or(host));
    let store = normalize_cache_key_part(store.unwrap_or("default"));
    let variant = normalize_cache_key_part(variant.unwrap_or("default"));
    normalize_cache_key(&format!(
        "{}:site:{}:store:{}:variant:{}",
        base, site, store, variant
    ))
}

fn normalize_cache_key_part(raw: &str) -> String {
    let normalized = normalize_cache_key(raw);
    if normalized.is_empty() {
        "default".to_string()
    } else if normalized.len() > 64 {
        normalized[..64].to_string()
    } else {
        normalized
    }
}

fn normalize_path(path: &str) -> String {
    let path = if path.is_empty() { "/" } else { path };
    let mut normalized = String::with_capacity(path.len());
    let mut last_was_slash = false;

    for ch in path.chars() {
        if ch == '/' {
            if !last_was_slash {
                normalized.push('/');
            }
            last_was_slash = true;
        } else {
            normalized.push(ch);
            last_was_slash = false;
        }
    }

    if normalized.is_empty() {
        "/".to_string()
    } else if normalized != "/" {
        normalized.trim_end_matches('/').to_string()
    } else {
        normalized
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_io_error(err: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}

fn hit_rate(hits: u64, misses: u64) -> f64 {
    let total = hits + misses;
    if total == 0 {
        0.0
    } else {
        (hits as f64 / total as f64) * 100.0
    }
}

fn avg_latency_ms(total_micros: u64, ops: u64) -> f64 {
    if ops == 0 {
        0.0
    } else {
        (total_micros as f64 / ops as f64) / 1000.0
    }
}

/// Parse size string (e.g., "512M", "2G") to bytes
fn parse_size(s: &str) -> u64 {
    let s = s.trim().to_uppercase();

    if let Some(num) = s.strip_suffix('G') {
        num.parse::<u64>().unwrap_or(1) * 1024 * 1024 * 1024
    } else if let Some(num) = s.strip_suffix('M') {
        num.parse::<u64>().unwrap_or(512) * 1024 * 1024
    } else if let Some(num) = s.strip_suffix('K') {
        num.parse::<u64>().unwrap_or(1) * 1024
    } else {
        s.parse::<u64>().unwrap_or(512 * 1024 * 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("512M"), 512 * 1024 * 1024);
        assert_eq!(parse_size("2G"), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1024K"), 1024 * 1024);
        assert_eq!(parse_size("1048576"), 1_048_576);
    }

    #[test]
    fn test_build_page_cache_key() {
        assert_eq!(
            build_page_cache_key("Example.com:8080", "//shop///products/"),
            "page:example.com:/shop/products"
        );
    }

    #[test]
    fn test_build_page_cache_key_scoped() {
        assert_eq!(
            build_page_cache_key_scoped(
                "Example.com:8080",
                Some("site-a"),
                Some("store-en"),
                Some("mobile"),
                "/catalog/a.html"
            ),
            "page:example.com:/catalog/a.html:site:site-a:store:store-en:variant:mobile"
        );
    }

    #[test]
    fn test_redis_payload_roundtrip_with_compression() {
        let entry = CacheEntry::new(
            vec![b'x'; 4096],
            "text/html".to_string(),
            vec!["domain:example.test".to_string()],
            Duration::from_secs(300),
            Duration::from_secs(120),
        );

        let encoded = RedisCacheLayer::serialize_entry(&entry).unwrap();
        let decoded = RedisCacheLayer::deserialize_entry(&encoded).unwrap();

        assert_eq!(decoded.data, entry.data);
        assert_eq!(decoded.content_type, entry.content_type);
        assert_eq!(decoded.tags, entry.tags);
        assert_eq!(decoded.ttl, entry.ttl);
        assert_eq!(decoded.stale_after, entry.stale_after);
    }

    #[tokio::test]
    async fn test_write_through_and_l1_hit() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let cache = CacheManager::new(&config);

        cache
            .set(
                "page:example.com:/",
                b"payload".to_vec(),
                "text/html",
                vec![],
            )
            .await;

        let first = cache.get("page:example.com:/").await;
        let second = cache.get("page:example.com:/").await;

        assert_eq!(first, Some(b"payload".to_vec()));
        assert_eq!(second, Some(b"payload".to_vec()));

        let stats = cache.stats();
        assert!(stats["l1"]["hits"].as_u64().unwrap_or(0) >= 2);
        assert!(stats["l2"]["writes"].as_u64().unwrap_or(0) >= 1);
    }

    #[tokio::test]
    async fn test_l2_fallback_promotes_to_l1() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let writer = CacheManager::new(&config);
        writer
            .set(
                "page:example.com:/l2",
                b"disk".to_vec(),
                "text/html",
                vec![],
            )
            .await;

        let reader = CacheManager::new(&config);
        let first = reader.get("page:example.com:/l2").await;
        let second = reader.get("page:example.com:/l2").await;

        assert_eq!(first, Some(b"disk".to_vec()));
        assert_eq!(second, Some(b"disk".to_vec()));

        let stats = reader.stats();
        assert!(stats["l2"]["hits"].as_u64().unwrap_or(0) >= 1);
        assert!(stats["l1"]["hits"].as_u64().unwrap_or(0) >= 1);
    }

    #[tokio::test]
    async fn test_stale_entry_is_not_served() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let cache = CacheManager::new(&config);
        cache
            .set_with_lifetime(
                "page:example.com:/stale",
                b"stale".to_vec(),
                "text/html",
                vec![],
                CacheLifetime::new(Duration::from_secs(10), Duration::from_secs(1)),
            )
            .await;

        tokio::time::sleep(Duration::from_secs(2)).await;

        let stale = cache.get("page:example.com:/stale").await;
        assert!(stale.is_none());

        let stats = cache.stats();
        assert!(
            stats["l1"]["stale"].as_u64().unwrap_or(0) >= 1
                || stats["l2"]["stale"].as_u64().unwrap_or(0) >= 1
        );
    }

    #[tokio::test]
    async fn test_layer_toggles() {
        let dir = tempdir().unwrap();
        let mut l1_only = CacheConfig::default();
        l1_only.disk_path = dir.path().to_string_lossy().to_string();
        l1_only.l1_enabled = true;
        l1_only.l2_enabled = false;

        let cache = CacheManager::new(&l1_only);
        cache
            .set("page:example.com:/l1", b"l1".to_vec(), "text/html", vec![])
            .await;
        assert_eq!(
            cache.get("page:example.com:/l1").await,
            Some(b"l1".to_vec())
        );

        let mut l2_only = CacheConfig::default();
        l2_only.disk_path = dir.path().to_string_lossy().to_string();
        l2_only.l1_enabled = false;
        l2_only.l2_enabled = true;

        let cache = CacheManager::new(&l2_only);
        cache
            .set(
                "page:example.com:/l2-only",
                b"l2".to_vec(),
                "text/html",
                vec![],
            )
            .await;
        assert_eq!(
            cache.get("page:example.com:/l2-only").await,
            Some(b"l2".to_vec())
        );
    }

    #[tokio::test]
    async fn test_remove_invalidates_l1_and_l2() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let cache = CacheManager::new(&config);
        cache
            .set(
                "page:example.com:/remove",
                b"gone".to_vec(),
                "text/html",
                vec![],
            )
            .await;
        assert_eq!(
            cache.get("page:example.com:/remove").await,
            Some(b"gone".to_vec())
        );

        cache.remove("page:example.com:/remove").await;
        assert!(cache.get("page:example.com:/remove").await.is_none());

        // New manager verifies L2 state on disk was also invalidated.
        let fresh_cache = CacheManager::new(&config);
        assert!(fresh_cache.get("page:example.com:/remove").await.is_none());
    }

    #[tokio::test]
    async fn test_purge_by_tag_evicts_only_matching_entries() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let cache = CacheManager::new(&config);
        cache
            .set(
                "page:example.com:/products/1",
                b"p1".to_vec(),
                "text/html",
                vec![
                    "domain:example.com".to_string(),
                    "category:shoes".to_string(),
                ],
            )
            .await;
        cache
            .set(
                "page:example.com:/products/2",
                b"p2".to_vec(),
                "text/html",
                vec!["domain:example.com".to_string()],
            )
            .await;
        cache
            .set(
                "page:other.com:/",
                b"other".to_vec(),
                "text/html",
                vec!["domain:other.com".to_string()],
            )
            .await;

        cache.purge_by_tag("category:shoes").await;

        assert!(cache.get("page:example.com:/products/1").await.is_none());
        assert_eq!(
            cache.get("page:example.com:/products/2").await,
            Some(b"p2".to_vec())
        );
        assert_eq!(cache.get("page:other.com:/").await, Some(b"other".to_vec()));
    }

    #[tokio::test]
    async fn test_purge_by_prefix_evicts_matching_keys() {
        let dir = tempdir().unwrap();
        let mut config = CacheConfig::default();
        config.disk_path = dir.path().to_string_lossy().to_string();
        config.l1_enabled = true;
        config.l2_enabled = true;

        let cache = CacheManager::new(&config);
        cache
            .set("page:example.com:/", b"home".to_vec(), "text/html", vec![])
            .await;
        cache
            .set(
                "page:example.com:/shop",
                b"shop".to_vec(),
                "text/html",
                vec![],
            )
            .await;
        cache
            .set("page:other.com:/", b"other".to_vec(), "text/html", vec![])
            .await;

        cache.purge_by_prefix("page:example.com:").await;

        assert!(cache.get("page:example.com:/").await.is_none());
        assert!(cache.get("page:example.com:/shop").await.is_none());
        assert_eq!(cache.get("page:other.com:/").await, Some(b"other".to_vec()));
    }
}
