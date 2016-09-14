//! The module that contains the request code.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::Error;
use std::str::from_utf8;
use std::sync::mpsc::channel;
use std::time::Duration;
use super::Method;
use curl::easy::{Easy, List};
use futures::{BoxFuture, failed, Future};
use response::Response;
use tokio_core::reactor::Handle;
use tokio_curl::Session;
use url::Url;

#[cfg(feature = "rustc-serialization")]
use rustc_serialize;

#[cfg(feature = "serde-serialization")]
use serde;
#[cfg(feature = "serde-serialization")]
use serde_json;

/// The default low byte rate threshold.
///
/// See [`Request::lowspeed_limit`](struct.Request.html#method.lowspeed_limit)
/// for more information.
pub const LOW_SPEED_LIMIT: u32 = 10;

/// The default low speed time threshold in seconds.
///
/// See [`Request::lowspeed_limit`](struct.Request.html#method.lowspeed_limit)
/// for more information.
pub const LOW_SPEED_TIME: u32 = 10;

/// The default redirect threshold for a single request.
///
/// cURL will follow this many redirects by default before aborting
/// the request. See [`Request::max_redirects`](struct.Request.html#method.max_redirects)
/// for more information.
pub const MAX_REDIRECTS: u32 = 15;

/// Represents an HTTP request.
///
/// While this can be used directly (and _must_ be for special HTTP verbs, it is
/// preferred to use the [`get`](fn.get.html), [`post`](fn.post.html), etc. functions
/// since they are shorter.
pub struct Request {
    body: Option<Vec<u8>>,
    follow_redirects: bool,
    handle: Option<Easy>,
    headers: HashMap<String, String>,
    lowspeed_limits: Option<(u32, Duration)>,
    max_redirects: u32,
    method: Method,
    params: HashMap<String, Vec<String>>,
    timeout: Option<Duration>,
    url: Url
}

impl Request {
    /// Creates a new instance of `Request`.
    pub fn new(url: &Url, method: Method) -> Self {
        Request {
            body: None,
            follow_redirects: true,
            handle: None,
            headers: HashMap::new(),
            lowspeed_limits: Some((LOW_SPEED_LIMIT, Duration::from_secs(LOW_SPEED_TIME as u64))),
            max_redirects: MAX_REDIRECTS,
            method: method,
            params: HashMap::new(),
            timeout: None,
            url: url.clone()
        }
    }

    /// Sets the body of the request as raw byte array.
    pub fn body(mut self, body: &AsRef<[u8]>) -> Self {
        self.body = Some(Vec::from(body.as_ref()));
        self
    }

    /// Sets the option whether to follow 3xx-redirects or not.
    ///
    /// Defaults to `true`.
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    /// Sets an HTTP header for the request. Remove headers by passing
    /// an empty value.
    ///
    /// ## Duplicates
    /// In spite of the W3C allowing multiple headers with the same name
    /// (https://www.w3.org/Protocols/rfc2616/rfc2616-sec4.html#sec4.2),
    /// we do not so that we get a cleaner and leaner API.
    ///
    /// If you really need to specify multiple header values for a single
    /// header, just set a comma-separated list here, as that, as per standards,
    /// is equivalent to sending multiple headers with the same name (see link).
    /// If your server code can't deal with that, go and burn. :P
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if value.is_empty() {
            self.headers.remove(name);
        } else {
            let value = value.to_owned();
            match self.headers.entry(name.to_owned()) {
                Entry::Occupied(mut e) => { e.insert(value); () }
                Entry::Vacant(e) => { e.insert(value); () }
            }
        }
        self
    }

    /// Serializes the given object to JSON and uses that as the request body.
    /// Also automatically sets the `Content-Type` to `application/json`.
    #[cfg(feature = "rustc-serialization")]
    pub fn json<T: rustc_serialize::Encodable>(self, body: &T) -> Self {
        self.set_json(rustc_serialize::json::encode(body).unwrap().into_bytes())
    }

    /// Serializes the given object to JSON and uses that as the request body.
    /// Also automatically sets the `Content-Type` to `application/json`.
    #[cfg(feature = "serde-serialization")]
    pub fn json<T: serde::Serialize>(self, body: &T) -> Self {
        self.set_json(serde_json::to_vec(body).unwrap())
    }

    /// Sets the thresholds which, when reached, aborts a download due to too
    /// low speeds.
    ///
    /// Pass 0 for either parameter to disable lowspeed limiting.
    ///
    /// ## Remarks
    /// `bytes` sets the minimum average amount of bytes transferred in `per_duration`
    /// time. If this number is not reached, cURL will abort the transfer because the transfer
    /// speed is too low.
    ///
    /// The values here default to [`LOW_SPEED_LIMIT`](constant.LOW_SPEED_LIMIT.html) and
    /// [`LOW_SPEED_TIME`](constant.LOW_SPEED_TIME.html).
    pub fn lowspeed_limit(mut self, bytes: u32, per_duration: Duration) -> Self {
        self.lowspeed_limits = if bytes > 0 && per_duration > Duration::from_secs(0) {
            Some((bytes, per_duration))
        } else {
            None
        };
        self
    }

    /// Sets the maximum amount of redirects cURL will follow when
    /// [`Request::follow_redirects`](#method.follow_redirects) is
    /// enabled.
    pub fn max_redirects(mut self, max_redirects: u32) -> Self {
        self.max_redirects = max_redirects;
        self
    }

    /// Adds a URL parameter to the request.
    ///
    /// ## Duplicates
    /// Duplicates are allowed to enable things like query parameters that use
    /// PHP array syntax (`&key[]=value`).
    pub fn param(mut self, name: &str, value: &str) -> Self {
        let value = value.to_owned();
        match self.params.entry(name.to_owned()) {
            Entry::Occupied(mut e) => e.get_mut().push(value),
            Entry::Vacant(e) => { e.insert(vec![value]); () }
        };
        self
    }

    /// Creates a new `Session` on the specified event loop to send the HTTP request through
    /// and returns a future that fires off the request, parses the response and resolves to
    /// a `Response`-struct on success.
    pub fn send(self, h: Handle) -> BoxFuture<Response, Error> {
        self.send_with_session(&Session::new(h))
    }

    /// Uses the given `Session` to send the HTTP request through and returns a future that
    /// fires off the request, parses the response and resolves to a `Response`-struct on success.
    pub fn send_with_session(mut self, session: &Session) -> BoxFuture<Response, Error> {
        {
            let mut query_pairs = self.url.query_pairs_mut();
            for (key, values) in self.params {
                for value in values {
                    query_pairs.append_pair(&key, &value);
                }
            }
        }
        let headers = {
            let mut list = List::new();
            for (key, value) in self.headers {
                list.append(&format!("{}: {}", key.trim(), value.trim())).expect("Failed to append header value to (native cURL) header list.");
            }
            list
        };

        let mut easy = self.handle.unwrap_or_else(|| Easy::new());
        let (header_tx, header_rx) = channel();
        let (body_tx, body_rx) = channel();

        let config_res = {
            // Make the borrow checker happy
            let body = self.body;
            let follow_redirects = self.follow_redirects;
            let lowspeed_limits = self.lowspeed_limits;
            let max_redirects = self.max_redirects;
            let method = self.method;
            let timeout = self.timeout;
            let url = self.url;
            let mut first_header = true;

            // We cannot use try! here, since we're dealing with futures, not with Results
            Ok(())
                .and_then(|_| easy.accept_encoding(""))
                .and_then(|_| easy.custom_request(method.as_ref()))
                .and_then(|_| if follow_redirects {
                    easy.follow_location(true)
                        .and_then(|_| easy.max_redirections(max_redirects))
                } else {
                    Ok(())
                })
                .and_then(|_| easy.header_function(move |header| {
                    match from_utf8(header) {
                        Ok(s) => {
                            let s = s.trim(); // Headers are \n-separated
                            if !first_header && s.len() > 0 { // First header is HTTP status line, don't want that
                                let _ = header_tx.send(s.to_owned());
                            }
                            first_header = false;
                            true
                        },
                        Err(_) => false
                    }
                }))
                .and_then(|_| easy.http_headers(headers))
                .and_then(|_| if let Some((bytes, per_time)) = lowspeed_limits {
                    easy.low_speed_limit(bytes)
                        .and_then(|_| easy.low_speed_time(per_time))
                } else {
                    Ok(())
                })
                .and_then(|_| if method == Method::Head {
                    easy.nobody(true)
                } else {
                    Ok(())
                })
                .and_then(|_| if let Some(ref body) = body {
                    easy.post_fields_copy(body)
                } else {
                    Ok(())
                })
                .and_then(|_| if let Some(timeout) = timeout {
                    easy.timeout(timeout)
                } else {
                    Ok(())
                })
                .and_then(|_| easy.url(url.as_str()))
                .and_then(|_| easy.write_function(move |data| {
                    let _ = body_tx.send(Vec::from(data));
                    Ok(data.len())
                }))
        };

        match config_res {
            Ok(_) => session.perform(easy)
                            .map(move |ez| {
                                let body = body_rx.try_iter().fold(Vec::new(), |mut data, slice| {
                                    data.extend(slice);
                                    data
                                });
                                let headers = header_rx.try_iter().collect::<Vec<_>>();
                                (ez, headers, body)
                            })
                            .map(|(ez, headers, body)| Response::new(ez, headers, body))
                            .map_err(|err| err.into_error())
                            .boxed(),
            Err(error) => failed(error.into()).boxed()
        }
    }

    /// Set the maximum time the request is allowed to take.
    ///
    /// Disabled by default in favor of [`lowspeed_limit`]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Uses the given cURL handle in the request process reusing its resources
    /// and improving performance.
    ///
    /// This is solely a way to improve performance, it is not necessary to call
    /// this method prior to firing off the request. The easy handle will be created
    /// automatically if necessary.
    pub fn use_handle(mut self, handle: Easy) -> Self {
        self.handle = Some(handle);
        self
    }

    #[cfg(any(feature = "rustc-serialization", feature = "serde-serialization"))]
    fn set_json(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self.header("Content-Type", "application/json")
    }
}

impl Debug for Request {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let len = if let Some(ref body) = self.body {
            body.len() as isize
        } else {
            -1isize
        };
        fmt.debug_struct(stringify!(Request))
            .field("body_len", &len)
            .field("follow_redirects", &self.follow_redirects)
            .field("headers", &self.headers)
            .field("method", &self.method)
            .field("params", &self.params)
            .field("reuses_handle", &self.handle.is_some())
            .field("url", &self.url)
            .finish()
    }
}

impl Display for Request {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{} {}", self.method, self.url)
    }
}

#[cfg(test)]
mod tests {
    use ::{Method, Request};
    use url::Url;

    #[cfg(feature = "rustc-serialization")]
    use rustc_serialize;

    #[cfg(feature = "serde-serialization")]
    use serde_json;

    #[cfg_attr(feature = "rustc-serialization", derive(RustcEncodable, RustcDecodable))]
    #[cfg_attr(feature = "serde-serialization", derive(Serialize, Deserialize))]
    struct TestPayload {
        a: u32,
        b: u32
    }

    #[test]
    fn test_payload() {
        let r = Request::new(&Url::parse("http://google.com/").unwrap(), Method::Get)
            .body(&get_serialized_payload());
        assert!(r.body.is_some());
    }

    #[cfg(feature = "rustc-serialization")]
    fn get_serialized_payload() -> Vec<u8> {
        rustc_serialize::json::encode(&TestPayload { a: 10, b: 15 }).unwrap().into_bytes()
    }

    #[cfg(feature = "serde-serialization")]
    fn get_serialized_payload() -> Vec<u8> {
        serde_json::to_vec(&TestPayload { a: 10, b: 15}).unwrap()
    }
}