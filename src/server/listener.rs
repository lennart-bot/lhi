//! TCP listener

use crate::{
    server::{accept_connections, Handler},
    HttpError,
};
use rustls::internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use std::fs::File;
use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

/// Listen on TCP
pub fn listen(
    addr: &str,
    threads: u8,
    tls_config: ServerConfig,
    handler: Handler,
) -> Result<Vec<JoinHandle<()>>, HttpError> {
    // listen
    let listener = TcpListener::bind(addr).or_else(HttpError::from)?;
    let listener = Arc::new(RwLock::new(listener));
    let tls_config = Arc::new(tls_config);

    // start threads
    let mut handler_threads = Vec::new();
    (0..threads).for_each(|_| {
        let listener = listener.clone();
        let tls_config = tls_config.clone();
        handler_threads.push(thread::spawn(move || {
            accept_connections(listener, tls_config, handler)
        }));
    });

    // return threads
    Ok(handler_threads)
}

/// Add TLS certificate and private key to ServerConfig
pub fn load_certificate(cert_path: &str, key_path: &str) -> Result<ServerConfig, HttpError> {
    // create config
    let mut config = ServerConfig::new(NoClientAuth::new());

    // open certificate
    let mut cert_buf = BufReader::new(File::open(cert_path).or_else(HttpError::from)?);
    let cert = match certs(&mut cert_buf) {
        Ok(key) => key,
        Err(_) => return HttpError::from("broken certificate"),
    };

    // open private key
    let mut key_buf = BufReader::new(File::open(key_path).or_else(HttpError::from)?);
    let key = match rsa_private_keys(&mut key_buf) {
        Ok(key) => {
            // check if key exists
            if !key.is_empty() {
                key[0].clone()
            } else {
                // open private key
                let mut key_buf = BufReader::new(File::open(key_path).or_else(HttpError::from)?);
                match pkcs8_private_keys(&mut key_buf) {
                    Ok(key) => {
                        // check if key exists
                        if !key.is_empty() {
                            key[0].clone()
                        } else {
                            return HttpError::from("broken private key");
                        }
                    }
                    Err(_) => return HttpError::from("broken private key"),
                }
            }
        }
        Err(_) => {
            // open private key
            let mut key_buf = BufReader::new(File::open(key_path).or_else(HttpError::from)?);
            match pkcs8_private_keys(&mut key_buf) {
                Ok(key) => {
                    // check if key exists
                    if !key.is_empty() {
                        key[0].clone()
                    } else {
                        return HttpError::from("broken private key");
                    }
                }
                Err(_) => return HttpError::from("broken private key"),
            }
        }
    };

    // add certificate to config and return
    config.set_single_cert(cert, key).or_else(HttpError::from)?;
    Ok(config)
}
