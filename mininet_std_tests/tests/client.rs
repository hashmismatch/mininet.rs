extern crate mininet_base;
extern crate mininet_http_client;

use mininet_http_client::HttpClientError;
use mininet_base::std::StdTcpStack;
use mininet_base::stack::TcpStack;
use mininet_http_client::http_get;
use slog::{o, Drain};

#[tokio::test]
async fn main() -> Result<(), HttpClientError> {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(drain, o!());

    let mut stack = StdTcpStack;

    let resp = http_get(&logger, &mut stack, "http://www.example.com/").await;
    println!("response: {:?}", resp);

    Ok(())
}