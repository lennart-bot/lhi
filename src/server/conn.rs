//! HTTP connection handling

use crate::server::{respond, Handler, HttpRequest, ResponseContent, ResponseData, Stream};
use kern::Fail;
use rustls::{ServerConfig, ServerSession, Stream as RustlsStream};
use std::io::prelude::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::thread;

const ISE: ResponseData = ResponseData {
    headers: None,
    status: Some("500 Internal Server Error"),
};

/// Accept connections
pub fn accept_connections(
    listener: Arc<RwLock<TcpListener>>,
    tls_config: Arc<ServerConfig>,
    handler: Handler,
) {
    loop {
        // accept connection
        if let Ok((stream, _)) = listener.read().unwrap().accept() {
            // spawn new thread
            let tls_config = tls_config.clone();
            thread::spawn(move || {
                // handle connection
                handle_connection(stream, tls_config, handler).ok();
            });
        }
    }
}

/// Handle connection
pub fn handle_connection(
    mut stream: TcpStream,
    tls_config: Arc<ServerConfig>,
    handler: Handler,
) -> Result<(), Fail> {
    // create TLS connection
    let mut session = ServerSession::new(&tls_config);
    let mut stream = RustlsStream::new(&mut session, &mut stream);

    // read header
    if let Ok((header, rest)) = read_header(&mut stream) {
        // parse HTTP request and process
        let http_request = HttpRequest::from(&header, rest, &mut stream);
        let response = match handler(http_request) {
            Ok(response) => response,
            Err(err) => respond(
                ResponseContent::Text(err.to_string()),
                "text/plain",
                Some(ISE),
            ),
        };

        // respond
        stream.write_all(&response).or_else(Fail::from)?;
        stream.flush().or_else(Fail::from)?;
    }

    // done
    Ok(())
}

// Read until \r\n\r\n (just working, uncommented)
fn read_header(stream: &mut Stream) -> Result<(String, Vec<u8>), Fail> {
    let mut header = Vec::new();
    let mut rest = Vec::new();
    let mut buf = vec![0u8; 8192];

    'l: loop {
        let length = match stream.read(&mut buf) {
            Ok(length) => length,
            Err(err) => return Fail::from(err),
        };
        for (i, &c) in buf.iter().enumerate() {
            if c == b'\r' {
                if buf.len() < i + 4 {
                    let mut buf_temp = vec![0u8; buf.len() - (i + 4)];
                    match stream.read(&mut buf_temp) {
                        Ok(_) => {}
                        Err(err) => return Fail::from(err),
                    };
                    let buf2 = [&buf[..], &buf_temp[..]].concat();
                    if buf2[i + 1] == b'\n' && buf2[i + 2] == b'\r' && buf2[i + 3] == b'\n' {
                        header.append(&mut buf);
                        header.append(&mut buf_temp);
                        break 'l;
                    }
                } else if buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
                    for &b in buf.iter().take(i + 4) {
                        header.push(b);
                    }
                    for &b in buf.iter().take(length).skip(i + 4) {
                        rest.push(b);
                    }
                    break 'l;
                } else if i + 1 == buf.len() {
                    for &b in buf.iter().take(i + 4) {
                        header.push(b);
                    }
                    for &b in buf.iter().take(length).skip(i + 4) {
                        rest.push(b);
                    }
                }
            }
        }
    }
    Ok((
        match String::from_utf8(header) {
            Ok(header) => header,
            Err(err) => return Fail::from(err),
        },
        rest,
    ))
}
