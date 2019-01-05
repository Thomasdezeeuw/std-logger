#[macro_use]
extern crate log;
extern crate std_logger;

use std_logger::REQUEST_TARGET;

fn main() {
    // Initialize the logger.
    std_logger::init();

    // Fake the handling of a request.
    logger_middleware(Request {
        url: "/".to_owned(),
        method: "GET".to_owned(),
    });
    logger_middleware(Request {
        url: "/not_found".to_owned(),
        method: "GET".to_owned(),
    });
}

// Our fake HTTP request.
struct Request {
    url: String,
    method: String,
}

// Our fake HTTP response.
struct Response {
    status_code: u16,
    body: String,
}

fn logger_middleware(request: Request) -> Response {
    // Clone the url and method. Note: don't actually do this in an HTTP this is
    // rather wastefull to.
    let url = request.url.clone();
    let method = request.url.clone();

    // Call our handler.
    let response = http_handler(request);

    // Log the request using the special request target. This will log it to
    // standard out rather then standard error.
    info!(target: REQUEST_TARGET, "url = `{}`, method = `{}`, status_code = {}, body_size = {}",
          url, method, response.status_code, response.body.len());

    if response.status_code == 404 {
        error!("oh no we've routed the user to an unknown page");
    }

    response
}

fn http_handler(request: Request) -> Response {
    match (request.method.as_str(), request.url.as_str()) {
        ("GET", "/") => Response {
            status_code: 200,
            body: "Home page".to_owned(),
        },
        _ => Response {
            status_code: 404,
            body: "Not found".to_owned(),
        },
    }
}
