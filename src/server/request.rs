//! HTTP request parsing

use crate::server::{HttpSettings, Stream};
use kern::Fail;
use std::collections::BTreeMap;
use std::io::prelude::Read;
use std::str::from_utf8;

/// HTTP request method (GET or POST)
#[derive(Debug, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
}

/// HTTP request structure
#[derive(Debug)]
pub struct HttpRequest<'a> {
    method: HttpMethod,
    url: &'a str,
    headers: BTreeMap<String, &'a str>,
    get: BTreeMap<String, &'a str>,
    post: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl<'a> HttpRequest<'a> {
    /// Get HTTP request method
    pub fn method(&self) -> &HttpMethod {
        // return HTTP request method
        &self.method
    }

    /// Get URL
    pub fn url(&self) -> &str {
        // return URL
        self.url
    }

    /// Get headers map
    pub fn headers(&self) -> &BTreeMap<String, &str> {
        // return headers map
        &self.headers
    }

    /// Get GET parameters
    pub fn get(&self) -> &BTreeMap<String, &str> {
        // return GET parameters map
        &self.get
    }

    /// Get POST parameters
    pub fn post(&self) -> &BTreeMap<String, String> {
        // return POST parameters map
        &self.post
    }

    /// Get body
    pub fn body(&self) -> &[u8] {
        // return body string
        &self.body
    }

    /// Parse HTTP request
    pub fn from(
        raw_header: &'a str,
        mut raw_body: Vec<u8>,
        stream: &mut Stream,
        http_settings: &HttpSettings,
    ) -> Result<Self, Fail> {
        // split header
        let mut header = raw_header.lines();
        let mut reqln = header
            .next()
            .ok_or_else(|| Fail::new("Empty header"))?
            .split(' ');

        // parse method
        let method = if reqln
            .next()
            .ok_or_else(|| Fail::new("No method in header"))?
            == "POST"
        {
            HttpMethod::POST
        } else {
            HttpMethod::GET
        };

        // parse url and split raw get parameters
        let mut get_raw = "";
        let url = if let Some(full_url) = reqln.next() {
            let mut split_url = full_url.splitn(2, '?');
            let url = split_url
                .next()
                .ok_or_else(|| Fail::new("No URL in header"))?;
            if let Some(params) = split_url.next() {
                get_raw = params;
            }
            url
        } else {
            "/"
        };

        // parse headers
        let mut headers = BTreeMap::new();
        header.for_each(|hl| {
            let mut hls = hl.splitn(2, ':');
            if let (Some(key), Some(value)) = (hls.next(), hls.next()) {
                headers.insert(key.trim().to_lowercase(), value.trim());
            }
        });

        // get content length
        let buf_len = if let Some(buf_len) = headers.get("Content-Length") {
            Some(buf_len)
        } else {
            headers.get("content-length")
        };

        // read rest of body
        let mut body = Vec::new();
        if let Some(buf_len) = buf_len {
            // parse buffer length
            let con_len = buf_len
                .parse::<usize>()
                .ok()
                .ok_or_else(|| Fail::new("Content-Length is not of type usize"))?;

            // check if body size is ok.
            if con_len > http_settings.max_body_size {
                return Fail::from("Max body size exceeded");
            }

            // read body
            let mut read_fails = 0;
            while raw_body.len() < con_len {
                // read next buffer
                let mut rest_body = vec![0u8; http_settings.body_buffer];
                let length = stream
                    .read(&mut rest_body)
                    .ok()
                    .ok_or_else(|| Fail::new("Stream broken"))?;
                rest_body.truncate(length);
                raw_body.append(&mut rest_body);

                // check if didn't read fully
                if length < http_settings.body_buffer {
                    read_fails += 1;

                    // failed too often
                    if read_fails > http_settings.body_read_attempts {
                        return Fail::from("Read body failed too often");
                    }
                }
            }

            // TODO parse not UTF-8 body file upload (binary, etc.)
            body = raw_body;
        }

        // parse GET and POST parameters
        let get = parse_parameters(get_raw, |v| v)?;
        let body_utf8 = from_utf8(&body).unwrap_or_default();
        let post = parse_post(&headers, &body_utf8)?;

        // return request
        Ok(Self {
            method,
            url,
            headers,
            get,
            post,
            body,
        })
    }
}

/// Parse POST parameters to map
fn parse_post(
    headers: &BTreeMap<String, &str>,
    body: &str,
) -> Result<BTreeMap<String, String>, Fail> {
    match headers.get("content-type") {
        Some(&content_type_header) => {
            let mut content_type_header = content_type_header.split(';').map(|s| s.trim());
            let mut content_type = None;
            let boundary = content_type_header.find_map(|s| {
                if s.starts_with("boundary=") {
                    return s.split('=').nth(1);
                } else if content_type.is_none() {
                    content_type = Some(s);
                }
                None
            });
            match content_type {
                Some(content_type) => {
                    if content_type == "multipart/form-data" {
                        parse_post_upload(
                            body,
                            boundary.ok_or_else(|| Fail::new("post upload, but no boundary"))?,
                        )
                    } else {
                        parse_parameters(body, |v| v.to_string())
                    }
                }
                None => parse_parameters(body, |v| v.to_string()),
            }
        }
        None => parse_parameters(body, |v| v.to_string()),
    }
}

/// Parse POST upload to map
fn parse_post_upload(body: &str, boundary: &str) -> Result<BTreeMap<String, String>, Fail> {
    // parameters map
    let mut params = BTreeMap::new();
    // split body into sections
    for mut section in body.split(&format!("--{}\r\n", boundary)).skip(1) {
        // check if last section
        let last_sep = format!("--{}--\r\n", boundary);
        if section.ends_with(&last_sep) {
            // remove ending seperator from last section
            section = &section[..(section.len() - last_sep.len() - 2)];
        }

        // split lines (max 3)
        let mut lines = section.splitn(3, "\r\n");
        let mut next_line = || {
            lines
                .next()
                .ok_or_else(|| Fail::new("broken section in post body"))
        };

        // parse name
        let name = next_line()?
            .split(';')
            .map(|s| s.trim())
            .find_map(|s| {
                if s.starts_with("name=") {
                    let name = s.split('=').nth(1)?;
                    Some(&name[1..(name.len() - 1)])
                } else {
                    None
                }
            })
            .ok_or_else(|| Fail::new("missing name in post body section"))?;

        // get value
        next_line()?;
        let data_section = next_line()?;
        let mut data_lines = data_section.splitn(2, "\r\n");
        let next_data_line = data_lines
            .next()
            .ok_or_else(|| Fail::new("broken section in post body"))?;
        let value = if let Some(file_data_line) = data_lines.next() {
            if next_data_line == "" {
                file_data_line.to_string()
            } else if file_data_line == "" {
                next_data_line.to_string()
            } else {
                format!("{}\r\n{}", next_data_line, file_data_line)
            }
        } else {
            next_data_line.to_string()
        };

        // insert into map
        params.insert(name.to_lowercase(), value);
    }

    // return parameters map
    Ok(params)
}

/// Parse GET parameters to map
fn parse_parameters<'a, V>(
    raw: &'a str,
    process_value: fn(&'a str) -> V,
) -> Result<BTreeMap<String, V>, Fail> {
    // parameters map
    let mut params = BTreeMap::new();

    // split parameters by ampersand
    for p in raw.split('&') {
        // split key and value and add to map
        let mut ps = p.splitn(2, '=');
        params.insert(
            ps.next()
                .ok_or_else(|| Fail::new("broken x-www-form-urlencoded parameters"))?
                .trim()
                .to_lowercase(), // trimmed key
            // correct value type
            process_value(if let Some(value) = ps.next() {
                value.trim() // trimmed value
            } else {
                "" // no value, is option
            }),
        );
    }

    // return parameters map
    Ok(params)
}
