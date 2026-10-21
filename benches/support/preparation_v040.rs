// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Request preparation extracted from client::execute at ee0e71b.

use super::request::Operation;
use reqwest::{Client, Method, Request, header::CONTENT_TYPE};
use std::hint::black_box;
use url::Url;

pub fn prepare(client: &Client, operation: Operation, mut url: Url, encoded: String) -> Request {
    if operation != Operation::Post {
        url.set_query(Some(&encoded));
    }
    let mut body = (operation == Operation::Post).then_some(encoded);
    let mut request = client.request(
        if operation == Operation::Post {
            Method::POST
        } else {
            Method::GET
        },
        url.clone(),
    );
    if operation == Operation::Post {
        request = request
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body.take().unwrap());
    }
    let request = request.build().unwrap();
    // The reviewed retry loop retains this original URL across the send.
    black_box(url);
    request
}
