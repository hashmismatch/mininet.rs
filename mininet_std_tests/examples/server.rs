use std::time::Duration;

use mininet_base::std::StdTcpStack;
use mininet_base::stack::TcpStack;
use mininet_base::stack::TcpError;
use mininet_std_tests::StdEnv;
use slog::{o, Drain, info};
use mininet_http_server::http_server;
use mininet_base::resp::HttpResponseWriter;
use mininet_http_server::HttpContext;
use mininet_base::std::StdTcpSocket;

fn main() -> Result<(), TcpError> {

    let example = async {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let mut stack = StdTcpStack;

        let addr = mininet_base::addr::SocketAddrV4::new(mininet_base::addr::Ipv4Addr::new(127, 0, 0, 1), 8080);
        let listener = stack.create_socket_listener(addr.into()).await?;

        info!(logger, "Listening at {}", addr);

        async fn handle_request(mut ctx: HttpContext<StdTcpSocket>) {
            ctx.http_ok("text/html", "<h1>Hello world!</h1>").await;
            ()
        }

        let env = StdEnv;

        http_server(&logger, env, listener, handle_request, Some(Duration::from_secs(10))).await;

        Ok(())
    };

    async_std::task::block_on(example)
}