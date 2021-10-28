use meh_http_common::std::StdTcpStack;
use meh_http_common::stack::TcpStack;
use meh_http_common::stack::TcpError;
use meh_http_client::http_get;
use slog::{o, Drain, info};
use meh_http_server::http_server;
use meh_http_common::resp::HttpResponseWriter;
use meh_http_server::HttpContext;
use meh_http_common::std::StdTcpSocket;
use meh_http_server_rest::rest_handler;

fn main() -> Result<(), TcpError> {

    let example = async {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let mut stack = StdTcpStack;

        let addr = meh_http_common::addr::SocketAddrV4::new(meh_http_common::addr::Ipv4Addr::new(127, 0, 0, 1), 8080);
        let listener = stack.create_socket_listener(addr.into()).await?;

        info!(logger, "Listening at http://{}/", addr);

        async fn handle_request(mut ctx: HttpContext<StdTcpSocket>) {
            
            rest_handler(ctx).await;

        }

        http_server(&logger, listener, handle_request).await;

        Ok(())
    };

    async_std::task::block_on(example)
}