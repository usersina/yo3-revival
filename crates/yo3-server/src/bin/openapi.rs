//! Export the dashboard contract without starting a server, loading configuration or opening a database.
fn main() -> std::io::Result<()> {
    use std::io::Write;
    std::io::stdout().lock().write_all(yo3_server::dashboard::openapi_json().as_bytes())
}
