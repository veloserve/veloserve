//! TLS support for VeloServe
//!
//! Loads certificates from config (global [ssl] + per-vhost ssl_certificate/ssl_certificate_key)
//! and builds a rustls ServerConfig with SNI-based certificate resolution.

use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use tracing::{info, warn};

use crate::config::Config;

/// SNI-aware certificate resolver that picks the right cert per domain.
pub struct VeloServeCertResolver {
    default: Option<Arc<CertifiedKey>>,
    certs: std::collections::HashMap<String, Arc<CertifiedKey>>,
}

impl VeloServeCertResolver {
    pub fn from_config(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mut resolver = Self {
            default: None,
            certs: std::collections::HashMap::new(),
        };

        if let Some(ref ssl) = config.ssl {
            match load_certified_key(&ssl.cert, &ssl.key) {
                Ok(ck) => {
                    info!("Loaded global SSL cert from {}", ssl.cert);
                    resolver.default = Some(Arc::new(ck));
                }
                Err(e) => warn!("Failed to load global SSL cert: {}", e),
            }
        }

        for vhost in &config.virtualhost {
            if let (Some(ref cert_path), Some(ref key_path)) =
                (&vhost.ssl_certificate, &vhost.ssl_certificate_key)
            {
                match load_certified_key(cert_path, key_path) {
                    Ok(ck) => {
                        info!("Loaded SSL cert for {} from {}", vhost.domain, cert_path);
                        resolver.certs.insert(vhost.domain.clone(), Arc::new(ck));
                    }
                    Err(e) => warn!("Failed to load SSL cert for {}: {}", vhost.domain, e),
                }
            }
        }

        if resolver.default.is_none() && resolver.certs.is_empty() {
            return Err("No SSL certificates loaded".into());
        }

        Ok(resolver)
    }
}

impl ResolvesServerCert for VeloServeCertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            if let Some(ck) = self.certs.get(sni) {
                return Some(ck.clone());
            }
        }
        self.default.clone()
    }
}

pub fn build_tls_config(config: &Config) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let resolver = VeloServeCertResolver::from_config(config)?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    Ok(tls_config)
}

fn load_certified_key(
    cert_path: &str,
    key_path: &str,
) -> Result<CertifiedKey, Box<dyn std::error::Error>> {
    let cert_file = std::fs::File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .filter_map(|c| c.ok())
        .collect();

    if certs.is_empty() {
        return Err(format!("No certificates found in {}", cert_path).into());
    }

    let key_file = std::fs::File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let private_key = rustls_pemfile::private_key(&mut key_reader)?
        .ok_or_else(|| format!("No private key found in {}", key_path))?;

    let signing_key = rustls::crypto::ring::sign::any_supported_type(&private_key)?;

    Ok(CertifiedKey::new(certs, signing_key))
}

pub fn can_enable_tls(config: &Config) -> bool {
    if config.server.listen_ssl.is_none() {
        return false;
    }
    if let Some(ref ssl) = config.ssl {
        if Path::new(&ssl.cert).exists() && Path::new(&ssl.key).exists() {
            return true;
        }
    }
    config.virtualhost.iter().any(|v| {
        v.ssl_certificate.as_ref().map_or(false, |p| Path::new(p).exists())
            && v.ssl_certificate_key.as_ref().map_or(false, |p| Path::new(p).exists())
    })
}
