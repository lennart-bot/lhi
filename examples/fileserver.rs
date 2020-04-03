extern crate lhi;

use kern::Fail;
use lhi::server::{listen, load_certificate, respond, HttpSettings};
use std::fs::File;
use std::io::prelude::Read;

fn main() {
    let config = load_certificate("examples/cert.pem", "examples/key.pem").unwrap();
    let http_settings = HttpSettings::new();
    let listeners = listen("[::]:8480", 4, http_settings, config, |req| {
        let req = req?;
        let filename = req
            .get()
            .get("file")
            .ok_or_else(|| Fail::new("filename missing, try adding ?file=... to the url"))?;
        let mut file = File::open(filename).or_else(Fail::from)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).or_else(Fail::from)?;
        Ok(respond(buf.as_bytes(), "text/html", None))
    })
    .unwrap();
    for listener in listeners {
        listener.join().expect("listener thread crashed");
    }
}
