extern crate lhi;

use kern::Fail;
use lhi::server::{listen, load_certificate, respond, HttpOptions, ResponseContent, ResponseData};
use std::fs::File;
use std::io::prelude::Read;

fn main() {
    let config = load_certificate("examples/cert.pem", "examples/key.pem").unwrap();
    let mut http_options = HttpOptions::new();
    let _ = listen("[::]:8480", 4, http_options, config, |req| {
        let req = req?;
        let filename = req
            .get()
            .get("file")
            .ok_or_else(|| Fail::new("filename missing, try adding ?file=... to the url"))?;
        let mut file = File::open(filename).or_else(Fail::from)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).or_else(Fail::from)?;
        Ok(respond(
            ResponseContent::Text(buf),
            "text/plain",
            ResponseData::new(),
        ))
    });
    loop {}
}
