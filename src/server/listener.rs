//! TCP listener

use crate::server::{accept_connections, Handler, HttpSettings};
use kern::Fail;
use rustls::internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

/// Listen on TCP
pub fn listen<T: Send + Sync + 'static>(
    addr: &str,
    threads: u8,
    http_settings: HttpSettings,
    tls_config: ServerConfig,
    handler: Handler<T>,
    shared: Arc<RwLock<T>>,
) -> Result<Vec<JoinHandle<()>>, Fail> {
    // listen
    let listener = TcpListener::bind(addr).or_else(Fail::from)?;
    let listener = Arc::new(RwLock::new(listener));

    // config
    let http_settings = Arc::new(http_settings);
    let tls_config = Arc::new(tls_config);

    // start threads
    let mut handler_threads = Vec::new();
    (0..threads).for_each(|_| {
        // clones
        let listener = listener.clone();
        let http_settings = http_settings.clone();
        let tls_config = tls_config.clone();
        let shared = shared.clone();

        // spawn thread
        handler_threads.push(thread::spawn(move || {
            accept_connections(listener, http_settings, tls_config, handler, shared)
        }));
    });

    // return threads
    Ok(handler_threads)
}

/// Generate config with TLS certificate and private key
pub fn certificate_config(raw_cert: &[u8], raw_key: &[u8]) -> Result<ServerConfig, Fail> {
    // create config
    let mut config = ServerConfig::new(NoClientAuth::new());

    // open certificate
    let mut cert_buf = BufReader::new(raw_cert);
    let cert = match certs(&mut cert_buf) {
        Ok(key) => key,
        Err(_) => return Fail::from("broken certificate"),
    };

    // open private key
    let mut key_buf = BufReader::new(raw_key);
    let key = match rsa_private_keys(&mut key_buf) {
        Ok(key) => {
            // check if key exists
            if !key.is_empty() {
                key[0].clone()
            } else {
                // open private key
                let mut key_buf = BufReader::new(raw_key);
                match pkcs8_private_keys(&mut key_buf) {
                    Ok(key) => {
                        // check if key exists
                        if !key.is_empty() {
                            key[0].clone()
                        } else {
                            return Fail::from("broken private key");
                        }
                    }
                    Err(_) => return Fail::from("broken private key"),
                }
            }
        }
        Err(_) => {
            // open private key
            let mut key_buf = BufReader::new(raw_key);
            match pkcs8_private_keys(&mut key_buf) {
                Ok(key) => {
                    // check if key exists
                    if !key.is_empty() {
                        key[0].clone()
                    } else {
                        return Fail::from("broken private key");
                    }
                }
                Err(_) => return Fail::from("broken private key"),
            }
        }
    };

    // add certificate to config and return
    config.set_single_cert(cert, key).or_else(Fail::from)?;
    Ok(config)
}

/// Generate config with TLS certificate and private key from file
pub fn load_certificate(cert_path: &str, key_path: &str) -> Result<ServerConfig, Fail> {
    // open files
    let mut cert_file = File::open(cert_path).or_else(Fail::from)?;
    let mut key_file = File::open(key_path).or_else(Fail::from)?;

    // create buffers
    let mut cert_buf = Vec::new();
    let mut key_buf = Vec::new();

    // read files
    cert_file.read_to_end(&mut cert_buf).or_else(Fail::from)?;
    key_file.read_to_end(&mut key_buf).or_else(Fail::from)?;

    // generate config and return
    certificate_config(&cert_buf, &key_buf)
}
