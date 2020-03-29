//! TCP listener

use crate::server::{accept_connections, Handler, HttpOptions};
use kern::Fail;
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
    http_options: HttpOptions,
    tls_config: ServerConfig,
    handler: Handler,
) -> Result<Vec<JoinHandle<()>>, Fail> {
    // listen
    let listener = TcpListener::bind(addr).or_else(Fail::from)?;
    let listener = Arc::new(RwLock::new(listener));

    // config
    let http_options = Arc::new(http_options);
    let tls_config = Arc::new(tls_config);

    // start threads
    let mut handler_threads = Vec::new();
    (0..threads).for_each(|_| {
        // clones
        let listener = listener.clone();
        let http_options = http_options.clone();
        let tls_config = tls_config.clone();

        // spawn thread
        handler_threads.push(thread::spawn(move || {
            accept_connections(listener, http_options, tls_config, handler)
        }));
    });

    // return threads
    Ok(handler_threads)
}

/// Add TLS certificate and private key to ServerConfig
pub fn load_certificate(cert_path: &str, key_path: &str) -> Result<ServerConfig, Fail> {
    // create config
    let mut config = ServerConfig::new(NoClientAuth::new());

    // open certificate
    let mut cert_buf = BufReader::new(File::open(cert_path).or_else(Fail::from)?);
    let cert = match certs(&mut cert_buf) {
        Ok(key) => key,
        Err(_) => return Fail::from("broken certificate"),
    };

    // open private key
    let mut key_buf = BufReader::new(File::open(key_path).or_else(Fail::from)?);
    let key = match rsa_private_keys(&mut key_buf) {
        Ok(key) => {
            // check if key exists
            if !key.is_empty() {
                key[0].clone()
            } else {
                // open private key
                let mut key_buf = BufReader::new(File::open(key_path).or_else(Fail::from)?);
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
            let mut key_buf = BufReader::new(File::open(key_path).or_else(Fail::from)?);
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
