//! The module that contains the code handling the HTTP response.

use std::convert::From;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::str;

use curl::easy::Easy;
use mime::Mime;

#[cfg(feature = "rustc-serialization")]
use rustc_serialize;

#[cfg(feature = "serde-serialization")]
use serde;
#[cfg(feature = "serde-serialization")]
use serde_json;

#[cfg(any(feature = "rustc-serialization", feature = "serde-serialization"))]
use std::io::{Error, ErrorKind};

/// Represents an HTTP response.
pub struct Response {
    body: Vec<u8>,
    handle: Easy,
    headers: Vec<(String, String)>,
    status_code: u16
}

impl Response {
    /// Creates a `Response` from the results of a successful request.
    ///
    /// You usually don't create a response this way, but get one as result
    /// from `Request.send(...)`.
    pub fn new(mut easy: Easy, headers: Vec<String>, body: Vec<u8>) -> Response {
        let headers =  {
            let mut vec = Vec::new();
            for header in headers {
                let splitted: Vec<_> = header.splitn(2, ": ")
                                             .map(|part| part.trim())
                                             .filter(|part| part.len() > 0)
                                             .collect();
                if splitted.len() != 2 {
                    continue;
                }

                vec.push((splitted[0].to_owned(), splitted[1].to_owned()));
            }
            vec
        };
        let status_code = easy.response_code().expect("Failed to get the response status code from cURL.") as u16;
        Response {
            body: body,
            handle: easy,
            headers: headers,
            status_code: status_code
        }
    }

    /// Gets the response body's bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Gets a mutable reference to the response body's bytes.
    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.body
    }

    /// Attempts to read the body as UTF-8 string and returns the result.
    pub fn body_str(&self) -> Option<&str> {
        str::from_utf8(self.body()).ok()
    }

    /// Retreives the content type, if there is one.
    ///
    /// This function also returns none if there has been an error parsing
    /// the mime type.
    pub fn content_type(&self) -> Option<Mime> {
        self.header("Content-Type")
            .and_then(|h| h.parse::<Mime>().ok())
    }

    /// Attempts to get a single header value.
    ///
    /// If there are multiple headers with the same name, this method returns
    /// the first one. If you need to get access to the other values, use
    /// [`Response::headers()`](struct.Response.html#method.headers).
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.iter().filter(|kvp| kvp.0 == name)
                           .nth(0)
                           .map(|kvp| &kvp.1)
    }

    /// Gets all response headers.
    pub fn headers(&self) -> &Vec<(String, String)> {
        &self.headers
    }

    /// Checks whether the returned status code represents a success
    /// (HTTP status code 2xx) or not.
    pub fn is_success(&self) -> bool {
        match self.status_code {
            200...299 => true,
            _ => false
        }
    }

    /// Attempts to decode the response body from JSON to an
    /// object of the given type.
    ///
    /// Returns `ErrorKind::InvalidData` when the server response could not
    /// be read as UTF-8 string or if it could not be deserialized from JSON.
    #[cfg(feature = "rustc-serialization")]
    pub fn json<T: rustc_serialize::Decodable>(&self) -> Result<T, Error> {
        let string = try!(str::from_utf8(&self.body).map_err(|err| Error::new(ErrorKind::InvalidData, err)));
        rustc_serialize::json::decode(string).map_err(|err| Error::new(ErrorKind::InvalidData, err))
    }

    /// Attempts to decode the response body from JSON to an
    /// object of the given type.
    ///
    /// Returns `ErrorKind::InvalidData` when the server response could not
    /// be read as UTF-8 string or if it could not be deserialized from JSON.
    #[cfg(feature = "serde-serialization")]
    pub fn json<T: serde::Deserialize>(&self) -> Result<T, Error> {
        serde_json::from_slice(self.body()).map_err(|err| Error::new(ErrorKind::InvalidData, err))
    }

    /// Attempts to decode the response body from JSON into an abstract
    /// JSON representation.
    ///
    /// Returns `ErrorKind::InvalidData` when the server response could not
    /// be read as UTF-8 string or if it could not be deserialized from JSON.
    #[cfg(feature = "rustc-serialization")]
    pub fn json_value(&self) -> Result<rustc_serialize::json::Json, Error> {
        let string = try!(str::from_utf8(&self.body).map_err(|err| Error::new(ErrorKind::InvalidData, err)));
        rustc_serialize::json::Json::from_str(string).map_err(|err| Error::new(ErrorKind::InvalidData, err))
    }

    /// Attempts to decode the response body from JSON into an abstract
    /// JSON representation.
    ///
    /// Returns `ErrorKind::InvalidData` when the server response could not
    /// be read as UTF-8 string or if it could not be deserialized from JSON.
    #[cfg(feature = "serde-serialization")]
    pub fn json_value(&self) -> Result<serde_json::Value, Error> {
        self.json::<serde_json::Value>()
    }

    /// Consumes the response and returns the underlying cURL handle
    /// used for the request so that it can be reused.
    ///
    /// Calling `from()` or `into()` does the same.
    pub fn reuse(self) -> Easy {
        self.handle
    }

    /// Gets the response status code.
    pub fn status_code(&self) -> u16 {
        self.status_code
    }
}

impl Debug for Response {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.debug_struct(stringify!(Response))
            .field("body_str", &self.body_str())
            .field("headers", &self.headers)
            .field("status_code", &self.status_code)
            .finish()
    }
}

impl From<Response> for Easy {
    fn from(response: Response) -> Self {
        response.reuse()
    }
}

impl From<Response> for Vec<u8> {
    fn from(response: Response) -> Self {
        response.body
    }
}

#[cfg(feature = "response-to-string")]
impl ::std::convert::TryFrom<Response> for String {
    type Err = ::std::string::FromUtf8Error;

    fn try_from(response: Response) -> Result<Self, Self::Err> {
        String::from_utf8(response.body)
    }
}