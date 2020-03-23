extern crate lhi;

use lhi::server::{listen, load_certificate, respond, ResponseContent};
use lhi::HttpError;
use std::fs::File;
use std::io::prelude::Read;

fn main() {
    let config = load_certificate("cert.pem", "key.pem").unwrap();
    let _ = listen("[::]:8480", 4, config, |req| {
        let req = req?;
        let filename = req
            .get()
            .get("file")
            .ok_or_else(|| HttpError::new("filename missing, try adding ?file=... to the url"))?;
        let mut file = File::open(filename).or_else(HttpError::from)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).or_else(HttpError::from)?;
        Ok(respond(ResponseContent::Text(buf), "text/plain", None))
    });
    loop {}
}
