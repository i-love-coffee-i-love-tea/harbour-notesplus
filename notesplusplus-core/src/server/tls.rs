//! TLS support and self-signed certificate generation for Notes++ embedded web server.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig as RustlsServerConfig;
use serde::{Deserialize, Serialize};

use crate::error::NotesError;
pub use super::cert_gen::generate_self_signed_cert;

/// Holds the generated or loaded certificate and private key in PEM and DER formats.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsCertificate {
    pub cert_pem: String,
    pub key_pem: String,
    #[serde(skip)]
    pub cert_ders: Vec<Vec<u8>>,
    #[serde(skip)]
    pub key_der: Vec<u8>,
}

/// Metadata information about the active TLS certificate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsStatusInfo {
    pub is_tls: bool,
    pub is_custom: bool,
    pub cert_path: String,
    pub key_path: String,
    pub subject: String,
}

/// Options for TLS certificate generation.
#[derive(Clone, Debug)]
pub struct TlsOptions {
    pub common_name: String,
    pub organization: String,
    pub alt_names: Vec<String>,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

impl Default for TlsOptions {
    fn default() -> Self {
        Self {
            common_name: "Notes Plus Web Server".to_string(),
            organization: "Notes Plus".to_string(),
            alt_names: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            cert_path: None,
            key_path: None,
        }
    }
}

/// Helper to parse certificate chain from PEM string.
pub fn pem_to_cert_chain(pem_str: &str) -> Result<Vec<CertificateDer<'static>>, NotesError> {
    let pems = pem::parse_many(pem_str).map_err(|e| NotesError::Tls(format!("Failed to parse certificate PEM: {}", e)))?;
    let mut certs = Vec::new();
    for p in pems {
        let tag = p.tag();
        if tag == "CERTIFICATE" || tag.contains("CERTIFICATE") {
            certs.push(CertificateDer::from(p.contents().to_vec()));
        }
    }
    if certs.is_empty() {
        return Err(NotesError::Tls("No certificate blocks found in PEM data".to_string()));
    }
    Ok(certs)
}

/// Helper to parse private key from PEM string (supports PKCS#8, PKCS#1 RSA, and SEC1 EC keys).
pub fn pem_to_private_key(pem_str: &str) -> Result<PrivateKeyDer<'static>, NotesError> {
    let pems = pem::parse_many(pem_str).map_err(|e| NotesError::Tls(format!("Failed to parse private key PEM: {}", e)))?;
    for p in pems {
        let tag = p.tag();
        let contents = p.contents().to_vec();
        if tag == "PRIVATE KEY" {
            return Ok(PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(contents)));
        } else if tag == "RSA PRIVATE KEY" {
            return Ok(PrivateKeyDer::Pkcs1(rustls::pki_types::PrivatePkcs1KeyDer::from(contents)));
        } else if tag == "EC PRIVATE KEY" {
            return Ok(PrivateKeyDer::Sec1(rustls::pki_types::PrivateSec1KeyDer::from(contents)));
        }
    }
    Err(NotesError::Tls("No valid private key found in PEM (must be PKCS#8, RSA, or EC private key)".to_string()))
}

/// Validates that the provided certificate and key PEM strings form a valid, matching pair.
pub fn validate_tls_pair(cert_pem: &str, key_pem: &str) -> Result<TlsCertificate, NotesError> {
    let cert_ders = pem_to_cert_chain(cert_pem)?;
    let key_der = pem_to_private_key(key_pem)?;

    // Ensure ring crypto provider is set for rustls 0.23
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Verify rustls accepts this cert + key combination
    let _ = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_ders.clone(), key_der.clone_key())
        .map_err(|e| NotesError::Tls(format!("Certificate and private key validation failed: {}", e)))?;

    let raw_cert_ders = cert_ders.into_iter().map(|c| c.to_vec()).collect();
    let raw_key_der = match key_der {
        PrivateKeyDer::Pkcs8(k) => k.secret_pkcs8_der().to_vec(),
        PrivateKeyDer::Pkcs1(k) => k.secret_pkcs1_der().to_vec(),
        PrivateKeyDer::Sec1(k) => k.secret_sec1_der().to_vec(),
        _ => Vec::new(),
    };

    Ok(TlsCertificate {
        cert_pem: cert_pem.to_string(),
        key_pem: key_pem.to_string(),
        cert_ders: raw_cert_ders,
        key_der: raw_key_der,
    })
}

/// Marker file indicating a custom certificate is in use.
fn custom_cert_marker_path(cert_path: &Path) -> PathBuf {
    cert_path.parent().unwrap_or(cert_path).join(".custom_cert")
}

/// Checks if a custom (user-installed) certificate is currently active.
pub fn is_custom_cert_installed(cert_path: &Path) -> bool {
    custom_cert_marker_path(cert_path).is_file()
}

/// Installs a custom TLS certificate and private key.
/// Accepts either raw PEM string content or file paths to existing PEM files.
pub fn install_custom_tls_cert(
    cert_source: &str,
    key_source: &str,
    target_cert_path: &Path,
    target_key_path: &Path,
) -> Result<TlsCertificate, NotesError> {
    let cert_pem = if let Ok(p) = Path::new(cert_source).canonicalize() {
        if p.is_file() {
            fs::read_to_string(p).map_err(|e| NotesError::Io(e))?
        } else {
            cert_source.to_string()
        }
    } else if Path::new(cert_source).is_file() {
        fs::read_to_string(cert_source).map_err(|e| NotesError::Io(e))?
    } else {
        cert_source.to_string()
    };

    let key_pem = if let Ok(p) = Path::new(key_source).canonicalize() {
        if p.is_file() {
            fs::read_to_string(p).map_err(|e| NotesError::Io(e))?
        } else {
            key_source.to_string()
        }
    } else if Path::new(key_source).is_file() {
        fs::read_to_string(key_source).map_err(|e| NotesError::Io(e))?
    } else {
        key_source.to_string()
    };

    let cert = validate_tls_pair(&cert_pem, &key_pem)?;

    if let Some(parent) = target_cert_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = target_key_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    crate::page::atomic_write(target_cert_path, cert.cert_pem.as_bytes())
        .map_err(|e| NotesError::Io(e))?;
    crate::page::atomic_write(target_key_path, cert.key_pem.as_bytes())
        .map_err(|e| NotesError::Io(e))?;

    // Mark as custom certificate
    let _ = fs::write(custom_cert_marker_path(target_cert_path), "custom");

    Ok(cert)
}

/// Resets certificate to a fresh self-signed certificate.
pub fn reset_to_self_signed_cert(
    cert_path: &Path,
    key_path: &Path,
    options: Option<TlsOptions>,
) -> Result<TlsCertificate, NotesError> {
    let opts = options.unwrap_or_default();
    let cert = generate_self_signed_cert(&opts)?;

    if let Some(parent) = cert_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = key_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    crate::page::atomic_write(cert_path, cert.cert_pem.as_bytes())
        .map_err(|e| NotesError::Io(e))?;
    crate::page::atomic_write(key_path, cert.key_pem.as_bytes())
        .map_err(|e| NotesError::Io(e))?;

    // Remove custom marker if present
    let marker = custom_cert_marker_path(cert_path);
    if marker.exists() {
        let _ = fs::remove_file(marker);
    }

    Ok(cert)
}

/// Gets an existing TLS certificate from files or creates and saves a new self-signed certificate.
pub fn get_or_create_tls_cert(
    cert_path: &Path,
    key_path: &Path,
    options: Option<TlsOptions>,
) -> Result<TlsCertificate, NotesError> {
    // Ensure parent directory exists
    if let Some(dir) = cert_path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    if cert_path.is_file() && key_path.is_file() {
        if let (Ok(cert_pem), Ok(key_pem)) = (fs::read_to_string(cert_path), fs::read_to_string(key_path)) {
            if let Ok(cert) = validate_tls_pair(&cert_pem, &key_pem) {
                return Ok(cert);
            }
        }
    }

    reset_to_self_signed_cert(cert_path, key_path, options)
}

/// Builds a rustls `ServerConfig` from a `TlsCertificate`.
pub fn create_rustls_server_config(cert: &TlsCertificate) -> Result<Arc<RustlsServerConfig>, NotesError> {
    // Ensure ring crypto provider is set for rustls 0.23
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cert_ders: Vec<CertificateDer<'static>> = if !cert.cert_ders.is_empty() {
        cert.cert_ders.iter().map(|c| CertificateDer::from(c.clone())).collect()
    } else {
        pem_to_cert_chain(&cert.cert_pem)?
    };

    let key_der: PrivateKeyDer<'static> = if !cert.key_der.is_empty() {
        PrivateKeyDer::try_from(cert.key_der.clone())
            .or_else(|_| pem_to_private_key(&cert.key_pem))
            .map_err(|e| NotesError::Tls(format!("Invalid private key DER: {:?}", e)))?
    } else {
        pem_to_private_key(&cert.key_pem)?
    };

    let server_config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_ders, key_der)
        .map_err(|e| NotesError::Tls(format!("Failed to create TLS server config: {}", e)))?;

    Ok(Arc::new(server_config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_or_create_tls_cert_persistence() {
        let dir = tempdir().unwrap();
        let cert_path = dir.path().join("tls").join("server.crt");
        let key_path = dir.path().join("tls").join("server.key");

        // 1. First run: generates and saves to disk
        let cert1 = get_or_create_tls_cert(&cert_path, &key_path, None)
            .expect("First generation should succeed");
        assert!(cert_path.exists());
        assert!(key_path.exists());
        assert!(!is_custom_cert_installed(&cert_path));

        // 2. Second run: loads existing files from disk
        let cert2 = get_or_create_tls_cert(&cert_path, &key_path, None)
            .expect("Loading existing cert should succeed");
        assert_eq!(cert1.cert_pem, cert2.cert_pem);
        assert_eq!(cert1.key_pem, cert2.key_pem);
    }

    #[test]
    fn test_install_custom_tls_cert_and_reset() {
        let dir = tempdir().unwrap();
        let cert_path = dir.path().join("tls").join("server.crt");
        let key_path = dir.path().join("tls").join("server.key");

        // Generate initial self-signed
        let _ = get_or_create_tls_cert(&cert_path, &key_path, None).unwrap();
        assert!(!is_custom_cert_installed(&cert_path));

        // Generate a different "custom" cert
        let custom_opts = TlsOptions {
            common_name: "custom.domain.com".to_string(),
            organization: "Custom Org".to_string(),
            alt_names: vec!["custom.domain.com".to_string()],
            cert_path: None,
            key_path: None,
        };
        let custom_cert = generate_self_signed_cert(&custom_opts).unwrap();

        // Install it
        let installed = install_custom_tls_cert(
            &custom_cert.cert_pem,
            &custom_cert.key_pem,
            &cert_path,
            &key_path,
        ).expect("Installation of custom cert should succeed");

        assert!(is_custom_cert_installed(&cert_path));
        assert_eq!(installed.cert_pem, custom_cert.cert_pem);

        // Verify loaded cert matches custom cert
        let loaded = get_or_create_tls_cert(&cert_path, &key_path, None).unwrap();
        assert_eq!(loaded.cert_pem, custom_cert.cert_pem);

        // Reset to self-signed
        let reset = reset_to_self_signed_cert(&cert_path, &key_path, None).unwrap();
        assert!(!is_custom_cert_installed(&cert_path));
        assert_ne!(reset.cert_pem, custom_cert.cert_pem);
    }

    #[test]
    fn test_create_rustls_server_config() {
        let cert = generate_self_signed_cert(&TlsOptions::default())
            .expect("Certificate generation should succeed");
        let config = create_rustls_server_config(&cert);
        assert!(config.is_ok(), "rustls ServerConfig creation should succeed: {:?}", config.err());
    }
}
