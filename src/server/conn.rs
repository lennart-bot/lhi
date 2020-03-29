//! HTTP connection handling

use crate::{
    server::{respond, Handler, HttpOptions, HttpRequest, ResponseContent, ResponseData, Stream},
    version,
};
use kern::Fail;
use rustls::{ServerConfig, ServerSession, Stream as RustlsStream};
use std::io::prelude::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::thread;

/// Accept connections
pub fn accept_connections(
    listener: Arc<RwLock<TcpListener>>,
    http_options: Arc<HttpOptions>,
    tls_config: Arc<ServerConfig>,
    handler: Handler,
) {
    loop {
        // accept connection
        if let Ok((stream, _)) = listener.read().unwrap().accept() {
            // spawn new thread
            let http_options = http_options.clone();
            let tls_config = tls_config.clone();
            thread::spawn(move || {
                // handle connection
                handle_connection(stream, &http_options, tls_config, handler).ok();
            });
        }
    }
}

/// Handle connection
pub fn handle_connection(
    mut stream: TcpStream,
    http_options: &HttpOptions,
    tls_config: Arc<ServerConfig>,
    handler: Handler,
) -> Result<(), Fail> {
    // create TLS connection
    let mut session = ServerSession::new(&tls_config);
    let mut stream = RustlsStream::new(&mut session, &mut stream);

    // read header
    let response = match read_header(&mut stream, http_options) {
        Ok((header, rest)) => {
            // parse HTTP request and process
            let http_request = HttpRequest::from(&header, rest, &mut stream, http_options);
            match handler(http_request) {
                Ok(response) => response,
                Err(err) => respond(
                    ResponseContent::Text(err.to_string()),
                    "text/plain",
                    ResponseData::new().set_status("400 Bad Request"),
                ),
            }
        }
        Err(err) => respond(
            ResponseContent::Text(format!("<!DOCTYPE html><html><head><title>{0}</title></head><body><h3>HTTP server error</h3><p>{0}</p><hr><address>ltheinrich.de/lhi v{1}</address></body></html>", err, version())),
            "text/html",
            ResponseData::new().set_status("400 Bad Request"),
        ),
    };

    // respond
    stream.write_all(&response).or_else(Fail::from)?;
    stream.flush().or_else(Fail::from)?;

    // done
    Ok(())
}

// Read until \r\n\r\n (just working, uncommented)
fn read_header(stream: &mut Stream, http_options: &HttpOptions) -> Result<(String, Vec<u8>), Fail> {
    let mut header = Vec::new();
    let mut rest = Vec::new();
    let mut buf = vec![0u8; http_options.header_buffer];

    'l: loop {
        let length = stream.read(&mut buf).or_else(Fail::from)?;
        if header.len() + length > http_options.max_header_size {
            return Fail::from("Max header size exceeded");
        }
        let buf = &buf[0..length];
        'f: for (i, &c) in buf.iter().enumerate() {
            if c == b'\r' {
                if buf.len() < i + 4 {
                    let mut buf_temp = vec![0u8; i + 4 - buf.len()];
                    stream.read(&mut buf_temp).or_else(Fail::from)?;
                    let mut buf2 = [&buf[..], &buf_temp[..]].concat();
                    let header_end =
                        buf2[i + 1] == b'\n' && buf2[i + 2] == b'\r' && buf2[i + 3] == b'\n';
                    header.append(&mut buf2);
                    if header_end {
                        break 'l;
                    } else {
                        break 'f;
                    }
                } else if buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
                    let (split1, split2) = buf.split_at(i + 4);
                    header.extend_from_slice(split1);
                    rest.extend_from_slice(split2);
                    break 'l;
                }
            }
            if buf.len() == i + 1 {
                header.extend_from_slice(&buf);
            }
        }
    }
    println!("{}", String::from_utf8_lossy(&header));
    Ok((
        match String::from_utf8(header) {
            Ok(header) => header,
            Err(err) => return Fail::from(err),
        },
        rest,
    ))
}
