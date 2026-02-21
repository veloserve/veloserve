//! Apache Configuration Compatibility Module
//!
//! This module provides compatibility with Apache httpd.conf and vhost files,
//! allowing VeloServe to read and convert Apache configurations.
//!
//! Supported directives:
//! - <VirtualHost>, ServerName, ServerAlias, DocumentRoot
//! - SSLEngine, SSLCertificateFile, SSLCertificateKeyFile
//! - php_admin_value, php_admin_flag
//! - DirectoryIndex, ErrorLog, CustomLog
//! - <Directory>, <IfModule>, <Files>

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod parser;
pub mod converter;
pub mod errors;

pub use parser::ApacheConfigParser;
pub use converter::ApacheToVeloServeConverter;
pub use errors::{ApacheParseError, ParseResult};

/// Represents a parsed Apache VirtualHost configuration
#[derive(Debug, Clone, Default)]
pub struct ApacheVirtualHost {
    /// Server names (primary + aliases)
    pub server_names: Vec<String>,
    /// Document root path
    pub document_root: Option<PathBuf>,
    /// Port (80, 443, etc.)
    pub port: u16,
    /// SSL configuration
    pub ssl: Option<ApacheSslConfig>,
    /// PHP settings (php_admin_value, php_admin_flag)
    pub php_settings: HashMap<String, String>,
    /// Directory index files
    pub directory_index: Vec<String>,
    /// Error log path
    pub error_log: Option<PathBuf>,
    /// Custom log path
    pub custom_log: Option<PathBuf>,
    /// Additional directives
    pub directives: Vec<ApacheDirective>,
}

/// SSL configuration from Apache
#[derive(Debug, Clone)]
pub struct ApacheSslConfig {
    pub enabled: bool,
    pub certificate_file: Option<PathBuf>,
    pub certificate_key_file: Option<PathBuf>,
    pub certificate_chain_file: Option<PathBuf>,
    pub protocols: Vec<String>,
    pub cipher_suite: Option<String>,
}

/// Apache configuration directive
#[derive(Debug, Clone)]
pub enum ApacheDirective {
    /// <VirtualHost ...> block
    VirtualHost {
        addresses: Vec<String>,
        content: Vec<ApacheDirective>,
    },
    /// <Directory ...> block
    Directory {
        path: String,
        content: Vec<ApacheDirective>,
    },
    /// <IfModule ...> block
    IfModule {
        module: String,
        content: Vec<ApacheDirective>,
    },
    /// <Files ...> block
    Files {
        pattern: String,
        content: Vec<ApacheDirective>,
    },
    /// Simple key-value directive
    Simple {
        name: String,
        value: String,
    },
    /// Comment line
    Comment(String),
}

/// Main Apache configuration structure
#[derive(Debug, Clone, Default)]
pub struct ApacheConfig {
    /// Global directives
    pub global_directives: Vec<ApacheDirective>,
    /// Virtual hosts
    pub virtual_hosts: Vec<ApacheVirtualHost>,
    /// Includes (paths to additional config files)
    pub includes: Vec<PathBuf>,
    /// LoadModule directives
    pub modules: Vec<(String, PathBuf)>,
}

impl ApacheConfig {
    /// Parse Apache configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let parser = ApacheConfigParser::new();
        parser.parse_file(path)
    }

    /// Parse Apache configuration from string
    pub fn from_str(content: &str) -> ParseResult<Self> {
        let parser = ApacheConfigParser::new();
        parser.parse(content)
    }

    /// Get all virtual hosts for a specific domain
    pub fn get_vhost(&self, domain: &str) -> Option<&ApacheVirtualHost> {
        self.virtual_hosts
            .iter()
            .find(|vh| vh.server_names.iter().any(|name| name == domain))
    }

    /// Add an include path
    pub fn add_include<P: AsRef<Path>>(&mut self, path: P) {
        self.includes.push(path.as_ref().to_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_vhost() {
        let config = r#"
<VirtualHost *:80>
    ServerName example.com
    ServerAlias www.example.com
    DocumentRoot /var/www/html
</VirtualHost>
"#;

        let result = ApacheConfig::from_str(config);
        assert!(result.is_ok());
        
        let apache_config = result.unwrap();
        assert_eq!(apache_config.virtual_hosts.len(), 1);
        
        let vhost = &apache_config.virtual_hosts[0];
        assert!(vhost.server_names.contains(&"example.com".to_string()));
        assert!(vhost.server_names.contains(&"www.example.com".to_string()));
        assert_eq!(vhost.document_root, Some(PathBuf::from("/var/www/html")));
        assert_eq!(vhost.port, 80);
    }

    #[test]
    fn test_parse_ssl_vhost() {
        let config = r#"
<VirtualHost *:443>
    ServerName secure.example.com
    DocumentRoot /var/www/secure
    SSLEngine on
    SSLCertificateFile /etc/ssl/certs/example.crt
    SSLCertificateKeyFile /etc/ssl/private/example.key
</VirtualHost>
"#;

        let result = ApacheConfig::from_str(config);
        assert!(result.is_ok());
        
        let apache_config = result.unwrap();
        let vhost = &apache_config.virtual_hosts[0];
        
        assert!(vhost.ssl.is_some());
        let ssl = vhost.ssl.as_ref().unwrap();
        assert!(ssl.enabled);
        assert_eq!(ssl.certificate_file, Some(PathBuf::from("/etc/ssl/certs/example.crt")));
    }
}
