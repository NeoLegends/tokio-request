# tokio-request
[![Build Status](https://travis-ci.org/NeoLegends/tokio-request.svg?branch=master)](https://travis-ci.org/NeoLegends/tokio-request)
[![Coverage Status](https://coveralls.io/repos/github/NeoLegends/tokio-request/badge.svg?branch=master)](https://coveralls.io/github/NeoLegends/tokio-request?branch=master)

An asynchronous HTTP client library for Rust

As this isn't crates.io as of now, add the following to your Cargo.toml to use this:
```
[dependencies]
tokio-request = { git = "https://github.com/NeoLegends/tokio-request" }
```

## Examples
Asynchronously send an HTTP request on the specified loop:

```rust
use tokio_core::Loop;
use tokio_request::str::get;
use url::Url;

let mut evloop = Loop::new().unwrap();
let future = get("https://httpbin.org/get")
                .header("User-Agent", "tokio-request")
                .param("Hello", "This is Rust")
                .param("Hello2", "This is also from Rust")
                .send(evloop.pin());
let result = evloop.run(future).expect("HTTP Request failed!");
println!(
    "Site answered with status code {} and body\n{}",
    result.status_code(),
    result.body_str().unwrap_or("<No response body>")
);
```

POST some JSON to an API (data must be rustc-serializable):

```rust
use tokio_core::Loop;
use tokio_request::str::post;

let mut evloop = Loop::new().unwrap();
let future = post("https://httpbin.org/post")
                .json(&Data { a: 10, b: 15 })
                .send(evloop.pin());
let result = evloop.run(future).expect("HTTP Request failed!");
println!(
    "Site answered with status code {} and body\n{}",
    result.status_code(),
    result.body_str().unwrap_or("<No response body>")
);
```
