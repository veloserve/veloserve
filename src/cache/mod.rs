//! Cache Module
//!
//! Multi-layer caching system for VeloServe.

use crate::config::CacheConfig;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use serde_json::json;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Cache entry
#[derive(Clone)]
struct CacheEntry {
    /// Cached data
    data: Vec<u8>,

    /// Content type
    content_type: String,

    /// Cache tags for invalidation
    tags: Vec<String>,

    /// When the entry was created
    created_at: Instant,

    /// Time to live
    ttl: Duration,
}

impl CacheEntry {
    /// Check if entry has expired
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Cache statistics
struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    size_bytes: AtomicU64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            size_bytes: AtomicU64::new(0),
        }
    }
}

/// Cache manager
pub struct CacheManager {
    /// In-memory cache (hot data)
    memory_cache: DashMap<String, CacheEntry>,

    /// LRU cache for eviction
    lru: Mutex<LruCache<String, ()>>,

    /// Tag to keys mapping
    tag_index: DashMap<String, Vec<String>>,

    /// Cache configuration
    config: CacheConfig,

    /// Statistics
    stats: CacheStats,

    /// Maximum memory usage in bytes
    max_memory: u64,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: &CacheConfig) -> Self {
        let max_memory = parse_size(&config.memory_limit);
        let max_entries = NonZeroUsize::new(10000).unwrap();

        info!(
            "Initializing cache: storage={:?}, max_memory={}",
            config.storage, config.memory_limit
        );

        Self {
            memory_cache: DashMap::new(),
            lru: Mutex::new(LruCache::new(max_entries)),
            tag_index: DashMap::new(),
            config: config.clone(),
            stats: CacheStats::new(),
            max_memory,
        }
    }

    /// Get an entry from cache
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.config.enable {
            return None;
        }

        if let Some(entry) = self.memory_cache.get(key) {
            if entry.is_expired() {
                // Remove expired entry
                drop(entry);
                self.remove(key).await;
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            // Update LRU
            {
                let mut lru = self.lru.lock();
                lru.get(key);
            }

            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            debug!("Cache hit: {}", key);
            return Some(entry.data.clone());
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store an entry in cache
    pub async fn set(&self, key: &str, data: Vec<u8>, content_type: &str, tags: Vec<String>) {
        if !self.config.enable {
            return;
        }

        let ttl = Duration::from_secs(self.config.default_ttl);
        self.set_with_ttl(key, data, content_type, tags, ttl).await;
    }

    /// Store an entry with custom TTL
    pub async fn set_with_ttl(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
        tags: Vec<String>,
        ttl: Duration,
    ) {
        if !self.config.enable {
            return;
        }

        let data_size = data.len() as u64;

        // Check if we need to evict
        let current_size = self.stats.size_bytes.load(Ordering::Relaxed);
        if current_size + data_size > self.max_memory {
            self.evict_lru().await;
        }

        let entry = CacheEntry {
            data,
            content_type: content_type.to_string(),
            tags: tags.clone(),
            created_at: Instant::now(),
            ttl,
        };

        // Store in memory cache
        self.memory_cache.insert(key.to_string(), entry);

        // Update LRU
        {
            let mut lru = self.lru.lock();
            lru.put(key.to_string(), ());
        }

        // Update tag index
        for tag in tags {
            self.tag_index
                .entry(tag)
                .or_insert_with(Vec::new)
                .push(key.to_string());
        }

        self.stats.size_bytes.fetch_add(data_size, Ordering::Relaxed);
        debug!("Cache set: {} ({} bytes, ttl={:?})", key, data_size, ttl);
    }

    /// Remove an entry from cache
    pub async fn remove(&self, key: &str) {
        if let Some((_, entry)) = self.memory_cache.remove(key) {
            let size = entry.data.len() as u64;
            self.stats.size_bytes.fetch_sub(size, Ordering::Relaxed);

            // Remove from tag index
            for tag in &entry.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.retain(|k| k != key);
                }
            }
        }

        // Remove from LRU
        {
            let mut lru = self.lru.lock();
            lru.pop(key);
        }
    }

    /// Purge all entries with a specific tag
    pub async fn purge_by_tag(&self, tag: &str) {
        info!("Purging cache entries with tag: {}", tag);

        if let Some((_, keys)) = self.tag_index.remove(tag) {
            for key in keys {
                self.remove(&key).await;
            }
        }
    }

    /// Purge all cache entries
    pub async fn purge_all(&self) {
        info!("Purging all cache entries");

        self.memory_cache.clear();
        self.tag_index.clear();

        {
            let mut lru = self.lru.lock();
            lru.clear();
        }

        self.stats.size_bytes.store(0, Ordering::Relaxed);
        self.stats.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Evict least recently used entries
    async fn evict_lru(&self) {
        let mut evicted = 0;
        let target = self.max_memory * 8 / 10; // Evict to 80% capacity

        while self.stats.size_bytes.load(Ordering::Relaxed) > target {
            let key_to_evict = {
                let mut lru = self.lru.lock();
                lru.pop_lru().map(|(k, _)| k)
            };

            if let Some(key) = key_to_evict {
                self.remove(&key).await;
                evicted += 1;
            } else {
                break;
            }
        }

        if evicted > 0 {
            debug!("Evicted {} cache entries", evicted);
            self.stats.evictions.fetch_add(evicted, Ordering::Relaxed);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> serde_json::Value {
        json!({
            "enabled": self.config.enable,
            "entries": self.memory_cache.len(),
            "hits": self.stats.hits.load(Ordering::Relaxed),
            "misses": self.stats.misses.load(Ordering::Relaxed),
            "evictions": self.stats.evictions.load(Ordering::Relaxed),
            "size_bytes": self.stats.size_bytes.load(Ordering::Relaxed),
            "max_memory": self.max_memory,
            "hit_rate": self.hit_rate(),
        })
    }

    /// Calculate cache hit rate
    fn hit_rate(&self) -> f64 {
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let misses = self.stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
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

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("512M"), 512 * 1024 * 1024);
        assert_eq!(parse_size("2G"), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1024K"), 1024 * 1024);
        assert_eq!(parse_size("1048576"), 1048576);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(&config);

        // Test set and get
        cache
            .set("test_key", b"test_data".to_vec(), "text/plain", vec![])
            .await;

        let result = cache.get("test_key").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"test_data".to_vec());

        // Test remove
        cache.remove("test_key").await;
        assert!(cache.get("test_key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_tags() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(&config);

        // Set entries with tags
        cache
            .set(
                "product_1",
                b"data1".to_vec(),
                "text/plain",
                vec!["product".to_string(), "category_5".to_string()],
            )
            .await;

        cache
            .set(
                "product_2",
                b"data2".to_vec(),
                "text/plain",
                vec!["product".to_string()],
            )
            .await;

        // Purge by tag
        cache.purge_by_tag("category_5").await;

        // product_1 should be gone, product_2 should remain
        assert!(cache.get("product_1").await.is_none());
        assert!(cache.get("product_2").await.is_some());
    }
}

