use log::info;
use std_logger::request;

fn main() {
    // Initialize the logger.
    let kvs = ("hostname", "node01");
    std_logger::Config::logfmt().with_kvs(kvs).init();

    info!(target: "some_target", single = "value"; "some message");

    // Fake the handling of a request.
    logger_middleware(Request {
        url: "/".to_owned(),
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
    // rather wasteful.
    let url = request.url.clone();
    let method = request.method.clone();

    // Call our handler.
    let response = http_handler(request);

    request!(
        url = url,
        method = method,
        status_code = response.status_code,
        body_size = response.body.len();
        "got request",
    );

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
