//! Apache to VeloServe Configuration Converter
//!
//! Converts parsed Apache configuration to VeloServe TOML format.

use std::collections::HashMap;

use crate::apache_compat::{ApacheConfig, ApacheVirtualHost};
use crate::config::{Config, VirtualHostConfig};

/// Converts Apache configuration to VeloServe configuration
pub struct ApacheToVeloServeConverter {
    /// Default values for missing directives
    defaults: HashMap<String, String>,
    /// Enable strict mode (fail on unsupported directives)
    strict: bool,
}

impl ApacheToVeloServeConverter {
    /// Create a new converter
    pub fn new() -> Self {
        let mut defaults = HashMap::new();
        defaults.insert("port".to_string(), "80".to_string());
        defaults.insert("php_memory_limit".to_string(), "256M".to_string());
        defaults.insert("php_max_execution_time".to_string(), "30".to_string());

        Self {
            defaults,
            strict: false,
        }
    }

    /// Enable strict mode
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Convert Apache configuration to VeloServe Config
    pub fn convert(&self, apache: &ApacheConfig) -> Config {
        let mut config = Config::default();

        for apache_vhost in &apache.virtual_hosts {
            if let Ok(veloserve_vhost) = self.convert_vhost(apache_vhost) {
                config.virtualhost.push(veloserve_vhost);
            }
        }

        self.apply_global_php_settings(&mut config, apache);

        config
    }

    /// Convert single Apache VirtualHost to VeloServe VirtualHostConfig
    fn convert_vhost(&self, apache: &ApacheVirtualHost) -> Result<VirtualHostConfig, ConversionError> {
        let domain = apache.server_names.first()
            .cloned()
            .unwrap_or_default();

        let root = apache.document_root
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                if self.strict {
                    String::new()
                } else {
                    "/var/www/html".to_string()
                }
            });

        if root.is_empty() && self.strict {
            return Err(ConversionError::MissingDocumentRoot);
        }

        let platform = self.detect_platform(&root);

        let ssl_certificate = apache.ssl.as_ref()
            .and_then(|s| s.certificate_file.as_ref())
            .map(|p| p.to_string_lossy().to_string());

        let ssl_certificate_key = apache.ssl.as_ref()
            .and_then(|s| s.certificate_key_file.as_ref())
            .map(|p| p.to_string_lossy().to_string());

        Ok(VirtualHostConfig {
            domain,
            root,
            platform: Some(platform),
            ssl_certificate,
            ssl_certificate_key,
            cache: None,
            index: vec!["index.php".to_string(), "index.html".to_string()],
            error_pages: std::collections::HashMap::new(),
        })
    }

    /// Detect CMS/platform from document root path
    fn detect_platform(&self, docroot: &str) -> String {
        let path = std::path::Path::new(docroot);

        if path.join("wp-config.php").exists() {
            return "wordpress".to_string();
        }
        if path.join("app/etc/env.php").exists() {
            return "magento2".to_string();
        }
        if path.join("artisan").exists() {
            return "laravel".to_string();
        }

        "generic".to_string()
    }

    /// Apply global PHP settings from Apache config
    fn apply_global_php_settings(&self, _config: &mut Config, apache: &ApacheConfig) {
        for directive in &apache.global_directives {
            if let crate::apache_compat::ApacheDirective::Simple { name, value } = directive {
                if name == "php_admin_value" || name == "php_value" {
                    // Parse "php_admin_value memory_limit 512M" format
                    let parts: Vec<&str> = value.splitn(2, char::is_whitespace).collect();
                    if parts.len() == 2 {
                        match parts[0] {
                            "memory_limit" => {
                                // config.php.memory_limit = parts[1].to_string();
                            }
                            "max_execution_time" => {
                                // config.php.max_execution_time = parts[1].parse().unwrap_or(30);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Generate VeloServe TOML string from Apache config
    pub fn to_toml(&self, apache: &ApacheConfig) -> String {
        let config = self.convert(apache);
        
        // In full implementation, this would serialize Config to TOML
        // For now, return a template
        let mut output = String::from(
            "# VeloServe Configuration\n\
             # Converted from Apache httpd.conf\n\
             \n\
             [server]\n\
             listen = \"0.0.0.0:80\"\n\
             workers = \"auto\"\n\
             \n"
        );

        output.push_str(&self.vhosts_toml_fragment(&config.virtualhost));
        output
    }

    /// Output only [[virtualhost]] blocks for appending to an existing base config.
    pub fn to_toml_vhosts_only(&self, apache: &ApacheConfig) -> String {
        let config = self.convert(apache);
        self.vhosts_toml_fragment(&config.virtualhost)
    }

    fn vhosts_toml_fragment(&self, vhosts: &[VirtualHostConfig]) -> String {
        let mut output = String::new();
        for vhost in vhosts {
            output.push_str(&format!(
                "[[virtualhost]]\n\
                 domain = \"{}\"\n\
                 root = \"{}\"\n\
                 platform = \"{}\"\n",
                vhost.domain,
                vhost.root,
                vhost.platform.as_deref().unwrap_or("generic"),
            ));

            if let Some(ref cert) = vhost.ssl_certificate {
                output.push_str(&format!("ssl_certificate = \"{}\"\n", cert));
            }
            if let Some(ref key) = vhost.ssl_certificate_key {
                output.push_str(&format!("ssl_certificate_key = \"{}\"\n", key));
            }

            output.push('\n');

            let platform = vhost.platform.as_deref().unwrap_or("");
            if platform == "wordpress" || platform == "magento2" {
                output.push_str(
                    "[virtualhost.cache]\n\
                     enable = true\n\
                     ttl = 3600\n\
                     exclude = [\"/wp-admin/*\", \"/wp-login.php\"]\n\n"
                );
            }
        }
        output
    }
}

impl Default for ApacheToVeloServeConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversion errors
#[derive(Debug)]
pub enum ConversionError {
    MissingDocumentRoot,
    MissingServerName,
    InvalidSslConfiguration,
    UnsupportedDirective(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::MissingDocumentRoot => {
                write!(f, "VirtualHost missing DocumentRoot")
            }
            ConversionError::MissingServerName => {
                write!(f, "VirtualHost missing ServerName")
            }
            ConversionError::InvalidSslConfiguration => {
                write!(f, "Invalid SSL configuration")
            }
            ConversionError::UnsupportedDirective(d) => {
                write!(f, "Unsupported directive: {}", d)
            }
        }
    }
}

impl std::error::Error for ConversionError {}
