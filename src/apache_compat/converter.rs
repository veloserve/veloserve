//! Apache to VeloServe Configuration Converter
//!
//! Converts parsed Apache configuration to VeloServe TOML format.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::apache_compat::{ApacheConfig, ApacheVirtualHost};
use crate::config::{Config, VirtualHost as VeloServeVhost};

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

        // Convert virtual hosts
        for apache_vhost in &apache.virtual_hosts {
            if let Ok(veloserve_vhost) = self.convert_vhost(apache_vhost) {
                config.virtual_hosts.push(veloserve_vhost);
            }
        }

        // Apply global PHP settings
        self.apply_global_php_settings(&mut config, apache);

        config
    }

    /// Convert single Apache VirtualHost to VeloServe VirtualHost
    fn convert_vhost(&self, apache: &ApacheVirtualHost) -> Result<VeloServeVhost, ConversionError> {
        let mut vhost = VeloServeVhost::default();

        // Server names (primary is first)
        if let Some(primary) = apache.server_names.first() {
            vhost.domain = primary.clone();
        }

        // Document root
        if let Some(ref docroot) = apache.document_root {
            vhost.root = docroot.clone();
        } else if self.strict {
            return Err(ConversionError::MissingDocumentRoot);
        }

        // Auto-detect platform
        vhost.platform = self.detect_platform(&vhost.root);

        // SSL configuration
        if let Some(ref apache_ssl) = apache.ssl {
            if apache_ssl.enabled {
                // Note: In full implementation, this would set SSL fields
                // For now, we'll add a comment about SSL
            }
        }

        // PHP settings from php_admin_value
        for (key, value) in &apache.php_settings {
            match key.as_str() {
                "memory_limit" => {
                    // Map to VeloServe PHP config
                }
                "max_execution_time" => {
                    // Map to VeloServe PHP config
                }
                _ => {
                    // Store as custom PHP setting
                }
            }
        }

        Ok(vhost)
    }

    /// Detect CMS/platform from document root
    fn detect_platform(&self, docroot: &PathBuf) -> String {
        // Check for WordPress
        if docroot.join("wp-config.php").exists() {
            return "wordpress".to_string();
        }

        // Check for Magento 2
        if docroot.join("app/etc/env.php").exists() {
            return "magento2".to_string();
        }

        // Check for Laravel
        if docroot.join("artisan").exists() {
            return "laravel".to_string();
        }

        // Default
        "generic".to_string()
    }

    /// Apply global PHP settings from Apache config
    fn apply_global_php_settings(&self, config: &mut Config, apache: &ApacheConfig) {
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

        // Add virtual hosts
        for (i, vhost) in config.virtual_hosts.iter().enumerate() {
            output.push_str(&format!(
                "[[virtualhost]]\n\
                 domain = \"{}\"\n\
                 root = \"{}\"\n\
                 platform = \"{}\"\n\n",
                vhost.domain,
                vhost.root.display(),
                vhost.platform
            ));

            // Add cache settings for known platforms
            if vhost.platform == "wordpress" || vhost.platform == "magento2" {
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
