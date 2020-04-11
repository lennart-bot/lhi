extern crate lhi;

use kern::Fail;
use lhi::server::{listen, load_certificate, respond, unsecure::listen_redirect, HttpSettings};
use std::fs::File;
use std::io::prelude::Read;
use std::sync::{Arc, RwLock};

fn main() {
    let config = load_certificate("examples/cert.pem", "examples/key.pem").unwrap();
    let http_settings = HttpSettings::new();
    let listeners = listen(
        "[::]:8480",
        4,
        http_settings,
        config,
        |req, shared| {
            let mut num = shared.write().unwrap();
            *num += 1;
            dbg!(*num);
            let req = req?;
            let filename = req
                .get()
                .get("file")
                .ok_or_else(|| Fail::new("filename missing, try adding ?file=... to the url"))?;
            let mut file = File::open(filename).or_else(Fail::from)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf).or_else(Fail::from)?;
            Ok(respond(buf, "text/html", None))
        },
        Arc::new(RwLock::new(0u32)),
    )
    .unwrap();
    listen_redirect("[::]:8080", "localhost:8480".to_string()).unwrap();
    for listener in listeners {
        listener.join().expect("listener thread crashed");
    }
}
