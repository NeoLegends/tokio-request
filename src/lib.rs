//! A simple _asynchronous_ HTTP client library built on cURL
//! that looks like jQuery AJAX or python's request.
//!
//! This crate tries to reduce the boilerplate code one has to write
//! for asynchronous HTTP in Rust. It does this by being much more
//! opinionated as e.g. `hyper` and relying on the brand new
//! `tokio_curl`-crate and `futures-rs`.
//!
//! # Quick Start
//! Asynchronously send an HTTP request on the specified loop:
//!
//! ```rust
//! # extern crate tokio_core;
//! # extern crate tokio_request;
//! # extern crate url;
//! use tokio_core::reactor::Core;
//! use tokio_request::str::get;
//!
//! # fn main() {
//! let mut evloop = Core::new().unwrap();
//! let future = get("https://httpbin.org/get")
//!                 .header("User-Agent", "tokio-request")
//!                 .param("Hello", "This is Rust")
//!                 .param("Hello2", "This is also from Rust")
//!                 .send(evloop.handle());
//! let result = evloop.run(future).expect("HTTP Request failed!");
//! # assert!(result.is_success());
//! println!(
//!     "Site answered with status code {} and body\n{}",
//!     result.status_code(),
//!     result.body_str().unwrap_or("<No response body>")
//! );
//! # }
//! ```
//!
//! POST some JSON to an API:
//!
//! ```rust
//! # #![feature(plugin)]
//! # extern crate tokio_core;
//! # extern crate tokio_request;
//! # extern crate url;
//! # #[cfg(feature = "rustc-serialization")]
//! # extern crate rustc_serialize;
//! # #[cfg(feature = "serde-serialization")]
//! # extern crate serde;
//! # #[cfg(feature = "serde-serialization")]
//! # extern crate serde_json;
//! use tokio_core::reactor::Core;
//! use tokio_request::str::post;
//! #
//! # #[cfg(feature = "rustc-serialization")]
//! # #[derive(RustcEncodable)]
//! # struct Data {
//! #     a: u32,
//! #     b: u32
//! # }
//! #
//! # #[cfg(feature = "serde-serialization")]
//! # #[derive(Serialize)]
//! # struct Data {
//! #     a: u32,
//! #     b: u32
//! # }
//!
//! # fn main() {
//! let mut evloop = Core::new().unwrap();
//! let future = post("https://httpbin.org/post")
//!                 .json(&Data { a: 10, b: 15 })
//!                 .send(evloop.handle());
//! let result = evloop.run(future).expect("HTTP Request failed!");
//! # assert!(result.is_success());
//! println!(
//!     "Site answered with status code {} and body\n{}",
//!     result.status_code(),
//!     result.body_str().unwrap_or("<No response body>")
//! );
//! # }
//! ```
//!
//! # Caveats
//! Right now the focus for this library is on interacting with REST
//! APIs that talk JSON, so this library is buffering the entire response
//! into memory. This means it is not recommended for downloading large
//! files from the internet. Streaming request / response bodies will be
//! added at a later stage when implementation and API details have been
//! figured out.

#![deny(missing_docs)]
#![feature(receiver_try_iter)]

extern crate curl;
extern crate futures;
extern crate mime;
extern crate tokio_core;
extern crate tokio_curl;
extern crate url;

#[cfg(feature = "rustc-serialization")]
extern crate rustc_serialize;

mod request;
mod response;

use std::fmt::{Display, Formatter, Result as FmtResult};
pub use self::request::*;
pub use self::response::*;
use url::Url;

/// Issue a GET-Request to the specified URL.
pub fn get(url: &Url) -> Request {
    request(url, Method::Get)
}

/// Issue a DELETE-Request to the specified URL.
pub fn delete(url: &Url) -> Request {
    request(url, Method::Delete)
}

/// Issue a POST-Request to the specified URL.
pub fn post(url: &Url) -> Request {
    request(url, Method::Post)
}

/// Issue a PUT-Request to the specified URL.
pub fn put(url: &Url) -> Request {
    request(url, Method::Put)
}

/// Issue a request with the specified method to the specified URL.
pub fn request(url: &Url, method: Method) -> Request {
    Request::new(url, method)
}

/// A submodule which allows the request builder functions to be
/// used with string slices instead of URLs for convenience.
pub mod str {
    use ::{Method, Request};
    use url::Url;

    /// Issue a GET-Request to the specified URL.
    pub fn get(url: &str) -> Request {
        super::get(&parse_url(url))
    }

    /// Issue a DELETE-Request to the specified URL.
    pub fn delete(url: &str) -> Request {
        super::delete(&parse_url(url))
    }

    /// Issue a POST-Request to the specified URL.
    pub fn post(url: &str) -> Request {
        super::post(&parse_url(url))
    }

    /// Issue a PUT-Request to the specified URL.
    pub fn put(url: &str) -> Request {
        super::put(&parse_url(url))
    }

    /// Issue a request with the specified method to the specified URL.
    pub fn request(url: &str, method: Method) -> Request {
        super::request(&parse_url(url), method)
    }

    fn parse_url(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    #[cfg(test)]
    mod tests {
        macro_rules! generate_http_tests {
            ($name:ident) => {
                #[test]
                fn $name() {
                    use super::$name;
                    use tokio_core::reactor::Core;

                    let mut evloop = Core::new().unwrap();
                    let request = $name(&format!("https://httpbin.org/{}", stringify!($name)))
                                     .header("User-Agent", "tokio-request")
                                     .param("Hello", "This is Rust")
                                     .param("Hello2", "This is also from Rust");
                    println!("{:?}", request);
                    let future = request.send(evloop.handle());
                    let result = evloop.run(future).expect("HTTP Request failed!");
                    println!("{:?}", result);
                    assert!(result.is_success());
                    assert!(result.body().len() > 0);
                    assert!(result.headers().len() > 0);
                }
            }
        }

        generate_http_tests!(get);
        generate_http_tests!(post);
        generate_http_tests!(put);
        generate_http_tests!(delete);

        #[test]
        #[cfg(feature = "rustc-serialization")]
        fn test_json() {
            use super::get;
            use tokio_core::reactor::Core;

            let mut evloop = Core::new().unwrap();
            let request = get("https://httpbin.org/get")
                             .header("User-Agent", "tokio-request")
                             .param("Hello", "This is Rust")
                             .param("Hello2", "This is also from Rust");
            println!("{:?}", request);
            let future = request.send(evloop.handle());
            let result = evloop.run(future).expect("HTTP Request failed!");
            println!("{:?}", result);
            assert!(result.is_success());
            assert!(result.body().len() > 0);
            assert!(result.headers().len() > 0);
            result.json_value().unwrap();
        }
    }
}

/// Represents an HTTP method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Method {
    /// GET
    Get,
    /// POST
    Post,
    /// PUT
    Put,
    /// DELETE
    Delete,
    /// HEAD
    Head,
    /// TRACE
    Trace,
    /// CONNECT
    Connect,
    /// PATCH
    Patch,
    /// OPTIONS
    Options,
    /// A custom HTTP header.
    Custom(String)
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        match *self {
            Method::Connect => "CONNECT",
            Method::Custom(ref m) => m.as_ref(),
            Method::Delete => "DELETE",
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Trace => "TRACE"
        }
    }
}

impl Default for Method {
    fn default() -> Self {
        Method::Get
    }
}

impl Display for Method {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.write_str(self.as_ref())
    }
}