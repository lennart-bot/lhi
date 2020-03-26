//! HTTP request parsing

use crate::server::Stream;
use kern::Fail;
use std::collections::BTreeMap;
use std::io::prelude::Read;

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
    get: BTreeMap<&'a str, &'a str>,
    body: String,
}

// HTTP request implementation
impl<'a> HttpRequest<'a> {
    pub fn from(
        raw_header: &'a str,
        mut raw_body: Vec<u8>,
        stream: &mut Stream,
    ) -> Result<Self, Fail> {
        // split header
        let mut header = raw_header.lines();
        let mut reqln = header
            .next()
            .ok_or_else(|| Fail::new("empty header"))?
            .split(' ');

        // parse method
        let method = if reqln
            .next()
            .ok_or_else(|| Fail::new("no method in header"))?
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
                .ok_or_else(|| Fail::new("no url in header"))?;
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
        let mut body = String::new();
        if let Some(buf_len) = buf_len {
            // parse buffer length
            let con_len = buf_len
                .parse::<usize>()
                .ok()
                .ok_or_else(|| Fail::new("content-length is not of type usize"))?;
            // read body
            while raw_body.len() < con_len {
                let mut rest_body = vec![0u8; 65536];
                let length = stream
                    .read(&mut rest_body)
                    .ok()
                    .ok_or_else(|| Fail::new("stream broken"))?;
                rest_body.truncate(length);
                raw_body.append(&mut rest_body);
            }
            // TODO parse not UTF-8 body file upload (binary, etc.)
            body = String::from_utf8(raw_body)
                .ok()
                .ok_or_else(|| Fail::new("body is not utf-8"))?;
        }

        // parse GET parameters and return
        let get = parse_parameters(get_raw)?;
        Ok(Self {
            method,
            url,
            headers,
            get,
            body,
        })
    }

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
    pub fn get(&self) -> &BTreeMap<&str, &str> {
        // return GET parameters map
        &self.get
    }

    /// Get body
    pub fn body(&self) -> &str {
        // return body string
        &self.body
    }

    /// Parse POST parameters to map (EVERY CALL)
    pub fn post(&self) -> Result<BTreeMap<&str, &str>, Fail> {
        match self.headers.get("content-type") {
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
                                &self.body,
                                boundary
                                    .ok_or_else(|| Fail::new("post upload, but no boundary"))?,
                            )
                        } else {
                            parse_parameters(&self.body)
                        }
                    }
                    None => parse_parameters(&self.body),
                }
            }
            None => parse_parameters(&self.body),
        }
    }
}

// Parse POST upload to map
fn parse_post_upload<'a>(
    body: &'a str,
    boundary: &str,
) -> Result<BTreeMap<&'a str, &'a str>, Fail> {
    // parameters map
    let mut params = BTreeMap::new();
    // split body into sections
    for section in body.split(&format!("--{}\r\n", boundary)).skip(1) {
        // check if section ended
        if section == "--" {
            return Ok(params);
        }

        // split lines (max 4)
        let mut lines = section.splitn(4, "\r\n");
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
        let value = if next_line()? == "" {
            // next line is value
            next_line()?
        } else {
            // skip one line, next is value
            next_line()?;
            next_line()?
        };

        // insert into map
        params.insert(name, value);
    }

    // return parameters map
    Ok(params)
}

// Parse GET parameters to map
fn parse_parameters(raw: &str) -> Result<BTreeMap<&str, &str>, Fail> {
    // parameters map
    let mut params = BTreeMap::new();

    // split parameters by ampersand
    for p in raw.split('&') {
        // split key and value and add to map
        let mut ps = p.splitn(2, '=');
        params.insert(
            ps.next()
                .ok_or_else(|| Fail::new("broken x-www-form-urlencoded parameters"))?
                .trim(), // trimmed key
            if let Some(value) = ps.next() {
                value.trim() // trimmed value
            } else {
                "" // no value, is option
            },
        );
    }

    // return parameters map
    Ok(params)
}
