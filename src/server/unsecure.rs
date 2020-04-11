//! HTTP to HTTPS redirecter

use crate::server::redirect;
use kern::Fail;
use std::io::prelude::*;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread::spawn;
use std::thread::JoinHandle;
use std::time::Duration;

/// Redirect HTTP requests to HTTPS
///
/// unsecure_addr MUST be a listener address (e.g. localhost:80 or [::]:80)
///
/// secure_addr MUST be a target address (e.g. localhost or [::1]:443), https:// will be added automatically
pub fn listen_redirect(
    unsecure_addr: impl AsRef<str>,
    secure_addr: String,
) -> Result<JoinHandle<()>, Fail> {
    // listen
    let listener = TcpListener::bind(unsecure_addr.as_ref()).or_else(Fail::from)?;
    let secure_addr = Arc::new(secure_addr);

    // listener thread
    spawn(move || loop {
        // accept connections
        if let Ok((mut stream, _)) = listener.accept() {
            let secure_addr = secure_addr.clone();

            // handle connection
            spawn(move || {
                // set timeout
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();

                // create buffers
                let mut buf = Vec::new();
                let mut temp_buf = vec![0u8; 64];

                // read until \n (and further until temp_buf is filled again)
                while !buf.contains(&b'\n') && buf.len() < 2048 {
                    let len = stream.read(&mut temp_buf).unwrap();
                    buf.extend(&temp_buf[..len]);
                }

                // get first line
                let pos = buf.iter().position(|&b| b == b'\r').unwrap();
                buf.truncate(pos + 1);

                // split url
                let url = match String::from_utf8(buf) {
                    Ok(full_url) => full_url.split(' ').nth(1).unwrap().to_string(),
                    _ => panic!("Unsecure request line is not UTF-8"),
                };

                // write redirect
                stream
                    .write_all(&redirect(&format!("https://{}{}", &secure_addr, url)))
                    .unwrap();
                stream.flush().unwrap();
            });
        }
    })
    .join()
    .or_else(|_| Fail::from("Thread crashed"))
}
