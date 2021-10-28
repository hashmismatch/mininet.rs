extern crate meh_http_common;
extern crate meh_http_client;

use meh_http_client::HttpClientError;
use meh_http_common::std::StdTcpStack;
use meh_http_common::stack::TcpStack;

#[tokio::test]
async fn main() -> Result<(), HttpClientError> {
    let stack = StdTcpStack;
    let socket_addr = stack.get_socket_address("www.google.com:80").await?;
    println!("socket addr: {:?}", socket_addr);
    Ok(())
}