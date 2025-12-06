//! URL Router
//!
//! Simple URL routing for internal and API endpoints.

use std::collections::HashMap;

/// Route handler type
pub type RouteHandler = fn(&str) -> bool;

/// Simple URL router
pub struct Router {
    /// Exact match routes
    exact: HashMap<String, String>,

    /// Prefix match routes
    prefixes: Vec<(String, String)>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            exact: HashMap::new(),
            prefixes: Vec::new(),
        }
    }

    /// Add an exact match route
    pub fn add_exact(&mut self, path: &str, handler: &str) {
        self.exact.insert(path.to_string(), handler.to_string());
    }

    /// Add a prefix match route
    pub fn add_prefix(&mut self, prefix: &str, handler: &str) {
        self.prefixes.push((prefix.to_string(), handler.to_string()));
    }

    /// Match a path against routes
    pub fn match_path(&self, path: &str) -> Option<&str> {
        // Check exact matches first
        if let Some(handler) = self.exact.get(path) {
            return Some(handler);
        }

        // Check prefix matches
        for (prefix, handler) in &self.prefixes {
            if path.starts_with(prefix) {
                return Some(handler);
            }
        }

        None
    }

    /// Check if path matches any route
    pub fn matches(&self, path: &str) -> bool {
        self.match_path(path).is_some()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Route matching result
#[derive(Debug, Clone)]
pub struct RouteMatch {
    /// Handler name
    pub handler: String,

    /// Extracted parameters
    pub params: HashMap<String, String>,

    /// Remaining path after prefix
    pub remainder: String,
}

impl RouteMatch {
    /// Create a new route match
    pub fn new(handler: &str) -> Self {
        Self {
            handler: handler.to_string(),
            params: HashMap::new(),
            remainder: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let mut router = Router::new();
        router.add_exact("/health", "health_handler");
        router.add_exact("/api/status", "status_handler");

        assert_eq!(router.match_path("/health"), Some("health_handler"));
        assert_eq!(router.match_path("/api/status"), Some("status_handler"));
        assert_eq!(router.match_path("/other"), None);
    }

    #[test]
    fn test_prefix_match() {
        let mut router = Router::new();
        router.add_prefix("/api/v1/", "api_handler");
        router.add_prefix("/static/", "static_handler");

        assert_eq!(router.match_path("/api/v1/users"), Some("api_handler"));
        assert_eq!(router.match_path("/static/css/style.css"), Some("static_handler"));
        assert_eq!(router.match_path("/other"), None);
    }

    #[test]
    fn test_exact_takes_precedence() {
        let mut router = Router::new();
        router.add_exact("/api/v1/special", "special_handler");
        router.add_prefix("/api/v1/", "api_handler");

        assert_eq!(router.match_path("/api/v1/special"), Some("special_handler"));
        assert_eq!(router.match_path("/api/v1/users"), Some("api_handler"));
    }
}

