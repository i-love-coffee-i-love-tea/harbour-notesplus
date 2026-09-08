//! Self-signed X.509 certificate generation with pure-Rust ASN.1 DER encoding and ring ECDSA.

use std::net::IpAddr;

use ring::rand::SecureRandom;
use ring::signature::KeyPair;

use super::tls::{TlsCertificate, TlsOptions};

mod der {
    use std::net::IpAddr;

    pub fn tlv(tag: u8, val: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(val.len() + 4);
        out.push(tag);
        let len = val.len();
        if len < 128 {
            out.push(len as u8);
        } else if len < 256 {
            out.push(0x81);
            out.push(len as u8);
        } else if len < 65536 {
            out.push(0x82);
            out.push((len >> 8) as u8);
            out.push((len & 0xff) as u8);
        } else {
            out.push(0x83);
            out.push((len >> 16) as u8);
            out.push(((len >> 8) & 0xff) as u8);
            out.push((len & 0xff) as u8);
        }
        out.extend_from_slice(val);
        out
    }

    pub fn sequence(val: &[u8]) -> Vec<u8> {
        tlv(0x30, val)
    }

    pub fn set(val: &[u8]) -> Vec<u8> {
        tlv(0x31, val)
    }

    pub fn integer(val: &[u8]) -> Vec<u8> {
        if let Some(&first) = val.first() {
            if first & 0x80 != 0 {
                let mut v = Vec::with_capacity(val.len() + 1);
                v.push(0x00);
                v.extend_from_slice(val);
                return tlv(0x02, &v);
            }
        }
        tlv(0x02, val)
    }

    pub fn bit_string(val: &[u8]) -> Vec<u8> {
        let mut v = Vec::with_capacity(val.len() + 1);
        v.push(0x00); // 0 unused bits
        v.extend_from_slice(val);
        tlv(0x03, &v)
    }

    pub fn octet_string(val: &[u8]) -> Vec<u8> {
        tlv(0x04, val)
    }

    pub fn oid(val: &[u8]) -> Vec<u8> {
        tlv(0x06, val)
    }

    pub fn utf8_string(s: &str) -> Vec<u8> {
        tlv(0x0c, s.as_bytes())
    }

    pub fn utc_time(s: &str) -> Vec<u8> {
        tlv(0x17, s.as_bytes())
    }

    pub fn context_explicit(tag_num: u8, val: &[u8]) -> Vec<u8> {
        tlv(0xa0 | tag_num, val)
    }

    pub fn general_name_dns(dns: &str) -> Vec<u8> {
        tlv(0x82, dns.as_bytes())
    }

    pub fn general_name_ip(ip: &IpAddr) -> Vec<u8> {
        match ip {
            IpAddr::V4(v4) => tlv(0x87, &v4.octets()),
            IpAddr::V6(v6) => tlv(0x87, &v6.octets()),
        }
    }
}

/// Generates a new self-signed X.509 certificate and private key.
pub fn generate_self_signed_cert(options: &TlsOptions) -> Result<TlsCertificate, String> {
    let rng = ring::rand::SystemRandom::new();

    // 1. Generate ECDSA P-256 PKCS#8 document
    let pkcs8_doc = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &rng,
    )
    .map_err(|e| format!("Failed to generate ECDSA key pair: {:?}", e))?;

    let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8_doc.as_ref(),
        &rng,
    )
    .map_err(|e| format!("Failed to parse generated key pair: {:?}", e))?;

    let pub_key_bytes = key_pair.public_key().as_ref();

    // 2. Build TBSCertificate
    // Version: v3 (encoded as integer 2 inside [0] EXPLICIT)
    let version = der::context_explicit(0, &der::integer(&[2]));

    // Serial Number: 16 random bytes (positive integer)
    let mut serial_bytes = [0u8; 16];
    rng.fill(&mut serial_bytes)
        .map_err(|e| format!("Random generation failed: {:?}", e))?;
    serial_bytes[0] &= 0x7f; // Ensure MSB is 0 for positive integer
    if serial_bytes[0] == 0 {
        serial_bytes[0] = 1;
    }
    let serial = der::integer(&serial_bytes);

    // Signature Algorithm: ecdsa-with-SHA256 (1.2.840.10045.4.3.2)
    let sig_alg_oid = der::oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]);
    let sig_alg = der::sequence(&sig_alg_oid);

    // Issuer & Subject (Distinguished Name)
    let mut dn_entries = Vec::new();
    if !options.organization.is_empty() {
        // Organization OID: 2.5.4.10 -> 55 04 0A
        let o_seq = der::sequence(&[der::oid(&[0x55, 0x04, 0x0a]), der::utf8_string(&options.organization)].concat());
        dn_entries.extend_from_slice(&der::set(&o_seq));
    }
    if !options.common_name.is_empty() {
        // CommonName OID: 2.5.4.3 -> 55 04 03
        let cn_seq = der::sequence(&[der::oid(&[0x55, 0x04, 0x03]), der::utf8_string(&options.common_name)].concat());
        dn_entries.extend_from_slice(&der::set(&cn_seq));
    }
    let dn = der::sequence(&dn_entries);

    // Validity: notBefore (yesterday) to notAfter (10 years)
    let now = chrono::Utc::now();
    let not_before_str = (now - chrono::Duration::days(1)).format("%y%m%d%H%M%SZ").to_string();
    let not_after_str = (now + chrono::Duration::days(3650)).format("%y%m%d%H%M%SZ").to_string();
    let validity = der::sequence(&[der::utc_time(&not_before_str), der::utc_time(&not_after_str)].concat());

    // SubjectPublicKeyInfo: id-ecPublicKey (1.2.840.10045.2.1) + prime256v1 (1.2.840.10045.3.1.7)
    let spki_alg = der::sequence(
        &[
            der::oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
            der::oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
        ]
        .concat(),
    );
    let spki = der::sequence(&[spki_alg, der::bit_string(pub_key_bytes)].concat());

    // Extensions [3] EXPLICIT
    let mut san_entries = Vec::new();
    for name in &options.alt_names {
        if let Ok(ip) = name.parse::<IpAddr>() {
            san_entries.extend_from_slice(&der::general_name_ip(&ip));
        } else if !name.is_empty() {
            san_entries.extend_from_slice(&der::general_name_dns(name));
        }
    }
    if san_entries.is_empty() {
        san_entries.extend_from_slice(&der::general_name_dns("localhost"));
    }
    // SubjectAltName OID: 2.5.29.17 -> 55 1D 11
    let san_ext = der::sequence(
        &[
            der::oid(&[0x55, 0x1d, 0x11]),
            der::octet_string(&der::sequence(&san_entries)),
        ]
        .concat(),
    );

    // BasicConstraints OID: 2.5.29.19 -> 55 1D 13 (cA = FALSE)
    let bc_ext = der::sequence(
        &[
            der::oid(&[0x55, 0x1d, 0x13]),
            der::octet_string(&der::sequence(&[])),
        ]
        .concat(),
    );

    // KeyUsage OID: 2.5.29.15 -> 55 1D 0F (digitalSignature: bit 0 -> 7 unused bits, 0x80)
    let ku_ext = der::sequence(
        &[
            der::oid(&[0x55, 0x1d, 0x0f]),
            der::octet_string(&der::tlv(0x03, &[0x07, 0x80])),
        ]
        .concat(),
    );

    let ext_seq = der::sequence(&[san_ext, bc_ext, ku_ext].concat());
    let extensions = der::context_explicit(3, &ext_seq);

    // Assemble TBSCertificate
    let tbs_bytes = [
        version,
        serial,
        sig_alg.clone(),
        dn.clone(),
        validity,
        dn,
        spki,
        extensions,
    ]
    .concat();
    let tbs_der = der::sequence(&tbs_bytes);

    // 3. Sign TBSCertificate with key_pair
    let sig = key_pair
        .sign(&rng, &tbs_der)
        .map_err(|e| format!("Signing failed: {:?}", e))?;
    let sig_bit_str = der::bit_string(sig.as_ref());

    // 4. Assemble full X.509 Certificate DER
    let cert_der = der::sequence(&[tbs_der, sig_alg, sig_bit_str].concat());

    let cert_pem = pem::encode(&pem::Pem::new("CERTIFICATE", cert_der.clone()));
    let key_der = pkcs8_doc.as_ref().to_vec();
    let key_pem = pem::encode(&pem::Pem::new("PRIVATE KEY", key_der.clone()));

    Ok(TlsCertificate {
        cert_pem,
        key_pem,
        cert_ders: vec![cert_der],
        key_der,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tls::{create_rustls_server_config, validate_tls_pair};

    #[test]
    fn test_der_helpers() {
        // Test TLV length encodings
        let short = der::tlv(0x04, &[1, 2, 3]);
        assert_eq!(short, vec![0x04, 3, 1, 2, 3]);

        let medium_payload = vec![0xaa; 200];
        let medium = der::tlv(0x04, &medium_payload);
        assert_eq!(medium[0], 0x04);
        assert_eq!(medium[1], 0x81);
        assert_eq!(medium[2], 200);
        assert_eq!(&medium[3..], &medium_payload[..]);

        let long_payload = vec![0xbb; 300];
        let long = der::tlv(0x04, &long_payload);
        assert_eq!(long[0], 0x04);
        assert_eq!(long[1], 0x82);
        assert_eq!(long[2], 1); // 300 >> 8
        assert_eq!(long[3], 44); // 300 & 0xff
        assert_eq!(&long[4..], &long_payload[..]);

        // Test integer positive MSB padding
        let positive_without_msb = der::integer(&[0x05, 0x10]);
        assert_eq!(positive_without_msb, vec![0x02, 2, 0x05, 0x10]);

        let positive_with_msb = der::integer(&[0x80, 0x10]);
        assert_eq!(positive_with_msb, vec![0x02, 3, 0x00, 0x80, 0x10]);

        // Test general_name IP encodings
        let ipv4: IpAddr = "127.0.0.1".parse().unwrap();
        let ip_v4_der = der::general_name_ip(&ipv4);
        assert_eq!(ip_v4_der, vec![0x87, 4, 127, 0, 0, 1]);

        let ipv6: IpAddr = "::1".parse().unwrap();
        let ip_v6_der = der::general_name_ip(&ipv6);
        assert_eq!(ip_v6_der[0], 0x87);
        assert_eq!(ip_v6_der[1], 16);
        assert_eq!(&ip_v6_der[2..], &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

        // Test general_name DNS encoding
        let dns_der = der::general_name_dns("localhost");
        assert_eq!(dns_der, [vec![0x82, 9], b"localhost".to_vec()].concat());
    }

    #[test]
    fn test_generate_self_signed_cert() {
        let options = TlsOptions {
            common_name: "Notes++ Web Server".to_string(),
            organization: "Notes++".to_string(),
            alt_names: vec!["localhost".to_string(), "127.0.0.1".to_string(), "192.168.1.50".to_string()],
            cert_path: None,
            key_path: None,
        };

        let cert = generate_self_signed_cert(&options).expect("Certificate generation should succeed");
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.cert_pem.contains("END CERTIFICATE"));
        assert!(cert.key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(!cert.cert_ders.is_empty());
        assert!(!cert.key_der.is_empty());

        let validated = validate_tls_pair(&cert.cert_pem, &cert.key_pem);
        assert!(validated.is_ok(), "Generated cert and key pair must validate cleanly: {:?}", validated.err());
    }

    #[test]
    fn test_generate_cert_with_ipv6_and_various_sans() {
        let options = TlsOptions {
            common_name: "my-device.local".to_string(),
            organization: "Test Org".to_string(),
            alt_names: vec![
                "127.0.0.1".to_string(),
                "::1".parse::<IpAddr>().unwrap().to_string(),
                "2001:db8::1".to_string(),
                "notes.lan".to_string(),
                "sailfish.phone".to_string(),
            ],
            cert_path: None,
            key_path: None,
        };

        let cert = generate_self_signed_cert(&options).expect("Certificate generation with IPv6 and SANs should succeed");
        let validated = validate_tls_pair(&cert.cert_pem, &cert.key_pem);
        assert!(validated.is_ok(), "Validation failed: {:?}", validated.err());

        let server_config = create_rustls_server_config(&cert);
        assert!(server_config.is_ok(), "ServerConfig creation failed: {:?}", server_config.err());
    }

    #[test]
    fn test_generate_cert_empty_fields_fallback() {
        let options = TlsOptions {
            common_name: "".to_string(),
            organization: "".to_string(),
            alt_names: vec![],
            cert_path: None,
            key_path: None,
        };

        let cert = generate_self_signed_cert(&options).expect("Cert with empty options should fall back to valid cert");
        let validated = validate_tls_pair(&cert.cert_pem, &cert.key_pem);
        assert!(validated.is_ok(), "Empty fields cert validation failed: {:?}", validated.err());
    }

    #[test]
    fn test_cert_signature_cryptographic_verification() {
        let options = TlsOptions::default();
        let cert = generate_self_signed_cert(&options).expect("Certificate generation should succeed");
        let cert_der = &cert.cert_ders[0];

        // Parse key pair to retrieve public key
        let rng = ring::rand::SystemRandom::new();
        let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            &cert.key_der,
            &rng,
        )
        .expect("Key pair should parse from key_der");
        let pub_key_bytes = key_pair.public_key().as_ref();

        // Parse Certificate SEQUENCE
        // Certificate ::= SEQUENCE { tbsCertificate TBSCertificate, signatureAlgorithm AlgorithmIdentifier, signatureValue BIT STRING }
        assert_eq!(cert_der[0], 0x30, "Must start with SEQUENCE");
        
        // Helper to read tag + length
        fn read_tl(bytes: &[u8], offset: usize) -> (u8, usize, usize) {
            let tag = bytes[offset];
            let len_byte = bytes[offset + 1];
            if len_byte < 0x80 {
                (tag, len_byte as usize, offset + 2)
            } else {
                let num_len_bytes = (len_byte & 0x7f) as usize;
                let mut len = 0;
                for i in 0..num_len_bytes {
                    len = (len << 8) | (bytes[offset + 2 + i] as usize);
                }
                (tag, len, offset + 2 + num_len_bytes)
            }
        }

        let (_cert_tag, _cert_len, cert_content_start) = read_tl(cert_der, 0);

        // 1. Read TBSCertificate
        let (tbs_tag, tbs_len, tbs_content_start) = read_tl(cert_der, cert_content_start);
        assert_eq!(tbs_tag, 0x30, "TBSCertificate must be a SEQUENCE");
        let tbs_full_der = &cert_der[cert_content_start..(tbs_content_start + tbs_len)];

        // 2. Read Signature Algorithm
        let next_offset = tbs_content_start + tbs_len;
        let (sig_alg_tag, sig_alg_len, sig_alg_content_start) = read_tl(cert_der, next_offset);
        assert_eq!(sig_alg_tag, 0x30, "sig_alg must be a SEQUENCE");

        // 3. Read Signature BIT STRING
        let sig_offset = sig_alg_content_start + sig_alg_len;
        let (sig_tag, sig_len, sig_content_start) = read_tl(cert_der, sig_offset);
        assert_eq!(sig_tag, 0x03, "Signature must be a BIT STRING");
        assert_eq!(cert_der[sig_content_start], 0x00, "Bit string unused bits must be 0");
        let sig_asn1_bytes = &cert_der[(sig_content_start + 1)..(sig_content_start + sig_len)];

        // Cryptographically verify signature on TBSCertificate using public key
        let public_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ECDSA_P256_SHA256_ASN1,
            pub_key_bytes,
        );
        let verify_res = public_key.verify(tbs_full_der, sig_asn1_bytes);
        assert!(verify_res.is_ok(), "Cryptographic verification of certificate signature failed: {:?}", verify_res);
    }
}
