//! Apache Configuration Parser
//!
//! Parses Apache httpd.conf and vhost files into structured data.

use std::fs;
use std::path::Path;

use crate::apache_compat::{
    ApacheConfig, ApacheDirective, ApacheSslConfig, ApacheVirtualHost,
    errors::{ApacheParseError, ParseResult},
};

/// Parser for Apache configuration files
pub struct ApacheConfigParser {
    /// Enable verbose logging
    verbose: bool,
    /// Expand includes (Include, IncludeOptional directives)
    expand_includes: bool,
}

impl ApacheConfigParser {
    /// Create a new parser with default settings
    pub fn new() -> Self {
        Self {
            verbose: false,
            expand_includes: true,
        }
    }

    /// Enable/disable verbose logging
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable/disable expanding include directives
    pub fn expand_includes(mut self, expand: bool) -> Self {
        self.expand_includes = expand;
        self
    }

    /// Parse configuration from a file
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> ParseResult<ApacheConfig> {
        let content = fs::read_to_string(&path)
            .map_err(|e| ApacheParseError::IoError { 
                path: path.as_ref().to_path_buf(), 
                source: e 
            })?;
        
        self.parse(&content)
    }

    /// Parse configuration from string content
    pub fn parse(&self, content: &str) -> ParseResult<ApacheConfig> {
        let mut config = ApacheConfig::default();
        let mut lines = content.lines().peekable();
        let mut line_number = 0;

        while let Some(line) = lines.next() {
            line_number += 1;
            
            // Skip empty lines and comments (but keep them for context)
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Handle comments
            if trimmed.starts_with('#') {
                config.global_directives.push(
                    ApacheDirective::Comment(trimmed.to_string())
                );
                continue;
            }

            // Parse directive
            match self.parse_line(trimmed) {
                Ok(directive) => {
                    // Extract virtual hosts
                    if let ApacheDirective::VirtualHost { addresses, content } = &directive {
                        if let Ok(vhost) = self.parse_virtual_host(addresses, content) {
                            config.virtual_hosts.push(vhost);
                        }
                    } else if let ApacheDirective::Simple { name, value } = &directive {
                        // Handle global directives
                        match name.as_str() {
                            "Include" | "IncludeOptional" => {
                                if self.expand_includes {
                                    config.includes.push(PathBuf::from(value));
                                }
                            }
                            "LoadModule" => {
                                let parts: Vec<&str> = value.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    config.modules.push((
                                        parts[0].to_string(),
                                        PathBuf::from(parts[1]),
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    config.global_directives.push(directive);
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!("Warning at line {}: {:?}", line_number, e);
                    }
                    // Continue parsing even if one line fails
                }
            }
        }

        Ok(config)
    }

    /// Parse a single line into a directive
    fn parse_line(&self, line: &str) -> ParseResult<ApacheDirective> {
        // Handle block directives (<VirtualHost>, <Directory>, etc.)
        if line.starts_with('<') {
            return self.parse_block_start(line);
        }

        // Simple directive: Name value
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            return Err(ApacheParseError::EmptyDirective);
        }

        let name = parts[0].to_string();
        let value = parts.get(1).unwrap_or(&"").trim().to_string();

        Ok(ApacheDirective::Simple { name, value })
    }

    /// Parse block directive start
    fn parse_block_start(&self, line: &str) -> ParseResult<ApacheDirective> {
        // Extract block type and arguments
        let end_pos = line.find('>').ok_or(ApacheParseError::UnclosedBlock)?;
        let inner = &line[1..end_pos];
        
        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ApacheParseError::EmptyBlock);
        }

        let block_type = parts[0].to_lowercase();
        
        // For now, return a simplified version
        // In full implementation, we'd parse the entire block content
        match block_type.as_str() {
            "virtualhost" => {
                let addresses = parts[1..].iter().map(|s| s.to_string()).collect();
                Ok(ApacheDirective::VirtualHost {
                    addresses,
                    content: vec![], // Would be filled by parsing block content
                })
            }
            "directory" => {
                let path = parts.get(1).unwrap_or(&"/").to_string();
                Ok(ApacheDirective::Directory {
                    path,
                    content: vec![],
                })
            }
            "ifmodule" => {
                let module = parts.get(1).unwrap_or(&"").to_string();
                Ok(ApacheDirective::IfModule {
                    module,
                    content: vec![],
                })
            }
            "files" => {
                let pattern = parts.get(1).unwrap_or(&"").to_string();
                Ok(ApacheDirective::Files {
                    pattern,
                    content: vec![],
                })
            }
            _ => Err(ApacheParseError::UnknownBlock(block_type)),
        }
    }

    /// Parse VirtualHost block content into structured VirtualHost
    fn parse_virtual_host(
        &self,
        addresses: &[String],
        _content: &[ApacheDirective],
    ) -> ParseResult<ApacheVirtualHost> {
        let mut vhost = ApacheVirtualHost::default();

        // Extract port from address (e.g., "*:80" or "127.0.0.1:443")
        for addr in addresses {
            if let Some(port_str) = addr.split(':').nth(1) {
                if let Ok(port) = port_str.parse::<u16>() {
                    vhost.port = port;
                    break;
                }
            }
        }

        // Parse content directives
        for directive in _content {
            match directive {
                ApacheDirective::Simple { name, value } => {
                    match name.as_str() {
                        "ServerName" => {
                            if vhost.server_names.is_empty() {
                                vhost.server_names.push(value.clone());
                            }
                        }
                        "ServerAlias" => {
                            for alias in value.split_whitespace() {
                                vhost.server_names.push(alias.to_string());
                            }
                        }
                        "DocumentRoot" => {
                            vhost.document_root = Some(PathBuf::from(value));
                        }
                        "SSLEngine" => {
                            let enabled = value.eq_ignore_ascii_case("on");
                            if vhost.ssl.is_none() {
                                vhost.ssl = Some(ApacheSslConfig {
                                    enabled,
                                    ..Default::default()
                                });
                            } else if let Some(ref mut ssl) = vhost.ssl {
                                ssl.enabled = enabled;
                            }
                        }
                        "SSLCertificateFile" => {
                            if vhost.ssl.is_none() {
                                vhost.ssl = Some(ApacheSslConfig::default());
                            }
                            if let Some(ref mut ssl) = vhost.ssl {
                                ssl.certificate_file = Some(PathBuf::from(value));
                            }
                        }
                        "SSLCertificateKeyFile" => {
                            if vhost.ssl.is_none() {
                                vhost.ssl = Some(ApacheSslConfig::default());
                            }
                            if let Some(ref mut ssl) = vhost.ssl {
                                ssl.certificate_key_file = Some(PathBuf::from(value));
                            }
                        }
                        "DirectoryIndex" => {
                            vhost.directory_index = value
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect();
                        }
                        "ErrorLog" => {
                            vhost.error_log = Some(PathBuf::from(value));
                        }
                        "CustomLog" => {
                            // CustomLog has format: path format [env]
                            let path = value.split_whitespace().next()
                                .map(|s| PathBuf::from(s));
                            vhost.custom_log = path;
                        }
                        name if name.starts_with("php_admin_") => {
                            let key = name.strip_prefix("php_admin_").unwrap_or(name);
                            vhost.php_settings.insert(key.to_string(), value.clone());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(vhost)
    }
}

impl Default for ApacheConfigParser {
    fn default() -> Self {
        Self::new()
    }
}
