use log;
use std_logger::REQUEST_TARGET;

fn main() {
    // Initialize the logger.
    std_logger::init();

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
    // rather wastefull to.
    let url = request.url.clone();
    let method = request.method.clone();

    // Call our handler.
    let response = http_handler(request);

    log::info!("Hello world");

    let kvs: &[(&str, &dyn log::kv::ToValue)] = &[
        ("url", &&*url), // `String` -> `&&str` -> `&dyn ToValue`.
        ("method", &&*method),
        ("status_code", &response.status_code),
        ("body_size", &response.body.len()),
    ];

    let record = log::Record::builder()
        .args(format_args!("got request"))
        .level(log::Level::Info)
        .target(REQUEST_TARGET)
        .file(Some(file!()))
        .line(Some(line!()))
        .module_path(Some(module_path!()))
        .key_values(&kvs)
        .build();
    log::logger().log(&record);

    let record = log::Record::builder()
        .args(format_args!("some message"))
        .level(log::Level::Info)
        .target("some_target")
        .file(Some(file!()))
        .line(Some(line!()))
        .module_path(Some(module_path!()))
        .key_values(&("single", "value"))
        .build();
    log::logger().log(&record);

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
