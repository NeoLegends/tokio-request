use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::convert::From;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::{Error, ErrorKind};
use std::str;
use curl::easy::Easy;
use mime::Mime;

#[cfg(feature = "rustc-serialization")]
use rustc_serialize;

/// Represents an HTTP response.
pub struct Response {
    body: Vec<u8>,
    handle: Easy,
    headers: HashMap<String, String>,
    status_code: u16
}

impl Response {
    /// Creates a `Response` from the results of a successful request.
    ///
    /// You usually don't create a response this way, but get one as result
    /// from `Request.send(...)`.
    pub fn new(mut easy: Easy, headers: Vec<String>, body: Vec<u8>) -> Response {
        let headers =  {
            let mut map: HashMap<String, String> = HashMap::new();
            for header in headers {
                let splitted: Vec<_> = header.splitn(2, ": ")
                                             .map(|part| part.trim())
                                             .filter(|part| part.len() > 0)
                                             .collect();
                if splitted.len() != 2 {
                    continue;
                }

                // For every header, we check whether we've already got an entry
                // with it's key in our dictionary. If so, we append the new value
                // as per https://www.w3.org/Protocols/rfc2616/rfc2616-sec4.html#sec4.2
                // with a comma.
                match map.entry(splitted[0].to_owned()) {
                    Entry::Occupied(mut e) => {
                        let mut entry = e.get_mut();
                        entry.push_str(", ");
                        entry.push_str(splitted[1]);
                    },
                    Entry::Vacant(e) => {
                        e.insert(splitted[1].to_owned());
                    }
                }
            }
            map
        };
        let status_code = easy.response_code().unwrap() as u16;
        Response {
            body: body,
            handle: easy,
            headers: headers,
            status_code: status_code
        }
    }

    /// Gets the request body's bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
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
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// Gets the response headers.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Checks whether the returned status code represents a success
    /// (HTTP status code 200-299) or not.
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

    /// Consumes the response and returns the underlying cURL handle
    /// used for the request.
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