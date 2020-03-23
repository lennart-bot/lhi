//! HTTP response

use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ResponseContent {
    Text(String),
    Byte(Vec<u8>),
    StaticText(&'static str),
    StaticByte(&'static [u8]),
}

#[derive(Debug)]
pub struct ResponseData<'a> {
    pub status: Option<&'a str>,
    pub headers: Option<BTreeMap<&'a str, &'a str>>,
}

/// Create HTTP response
pub fn respond(
    content: ResponseContent,
    content_type: &str,
    data: Option<ResponseData>,
) -> Vec<u8> {
    // additional response data
    let mut status = "200 OK";
    let mut headers = String::new();
    if let Some(data) = data {
        if let Some(data_status) = data.status {
            status = data_status;
        }
        if let Some(data_headers) = data.headers {
            data_headers.iter().for_each(|(k, v)| {
                headers.push_str("\r\n");
                headers.push_str(k);
                headers.push_str(": ");
                headers.push_str(v);
            })
        }
    }

    // create response
    let mut response = Vec::new();
    let header = format!(
        "HTTP/1.1 {}\r\nserver: ltheinrich.de/lhi\r\ncontent-type: {}; charset=utf-8{}",
        status, content_type, headers
    );
    response.extend_from_slice(header.as_bytes());

    // write content
    match content {
        ResponseContent::Text(text) => {
            let content = text.as_bytes();
            response.append(&mut set_content_length(content.len()));
            response.extend_from_slice(content)
        }
        ResponseContent::Byte(byte) => {
            let content = &byte;
            response.append(&mut set_content_length(content.len()));
            response.extend_from_slice(content)
        }
        ResponseContent::StaticText(text) => {
            let content = text.as_bytes();
            response.append(&mut set_content_length(content.len()));
            response.extend_from_slice(content)
        }
        ResponseContent::StaticByte(content) => {
            response.append(&mut set_content_length(content.len()));
            response.extend_from_slice(content)
        }
    };
    response.extend_from_slice(b"\r\n");

    // return
    response
}

fn set_content_length(content_length: usize) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(b"\r\n");
    header.extend_from_slice(b"content-length: ");
    header.extend_from_slice((content_length + 2).to_string().as_bytes());
    header.extend_from_slice(b"\r\n\r\n");
    header
}

/// Create HTTP redirect response
pub fn redirect(url: &str) -> Vec<u8> {
    let mut headers = BTreeMap::new();
    headers.insert("location", url);
    let data = ResponseData {
        status: Some("303 See Other"),
        headers: Some(headers),
    };
    respond(
        ResponseContent::Text(format!("<html><head><title>Moved</title></head><body><h1>Moved</h1><p><a href=\"{0}\">{0}</a></p></body></html>", url)),
        "text/html",
        Some(data)
        )
}
