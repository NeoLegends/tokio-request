# tokio-request
[![Build Status](https://travis-ci.org/NeoLegends/tokio-request.svg?branch=master)](https://travis-ci.org/NeoLegends/tokio-request)
[![Coverage Status](https://coveralls.io/repos/github/NeoLegends/tokio-request/badge.svg?branch=master)](https://coveralls.io/github/NeoLegends/tokio-request?branch=master)

An asynchronous HTTP client library for Rust

As this isn't crates.io as of now, add the following to your Cargo.toml:
```
[dependencies]
tokio-request = { git = "https://github.com/NeoLegends/tokio-request" }
```

This library only works on Rust nightly at the moment.

## Examples
Asynchronously send an HTTP request on the specified loop:

```rust
use tokio_core::reactor::Core;
use tokio_request::str::get;
use url::Url;

let mut evloop = Core::new().unwrap();
let future = get("https://httpbin.org/get")
                .header("User-Agent", "tokio-request")
                .param("Hello", "This is Rust")
                .param("Hello2", "This is also from Rust")
                .send(evloop.handle());
let result = evloop.run(future).expect("HTTP Request failed!");
println!(
    "Site answered with status code {} and body\n{}",
    result.status_code(),
    result.body_str().unwrap_or("<No response body>")
);
```

POST some JSON to an API (data must be serializable):

```rust
use tokio_core::reactor::Core;
use tokio_request::str::post;

let mut evloop = Core::new().unwrap();
let future = post("https://httpbin.org/post")
                .json(&Data { a: 10, b: 15 })
                .send(evloop.handle());
let result = evloop.run(future).expect("HTTP Request failed!");
println!(
    "Site answered with status code {} and body\n{}",
    result.status_code(),
    result.body_str().unwrap_or("<No response body>")
);
```

## Caveats
Right now the focus for this library is on interacting with REST
APIs that talk JSON, so this library is buffering the entire response
into memory. This means it is not recommended for downloading large
files from the internet. Streaming request / response bodies will be
added at a later stage when implementation and API details have been
figured out.
