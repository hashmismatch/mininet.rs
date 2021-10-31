use std::sync::Arc;
use std::sync::Mutex;

use meh_http_client::http_get;
use meh_http_common::resp::HttpResponseWriter;
use meh_http_common::stack::TcpError;
use meh_http_common::stack::TcpStack;
use meh_http_common::std::StdTcpSocket;
use meh_http_common::std::StdTcpStack;
use meh_http_server::http_server;
use meh_http_server::HttpContext;
use meh_http_server_rest::allow_cors_all;
use meh_http_server_rest::not_found;
use meh_http_server_rest::quick_rest::quick_rest_value;
use meh_http_server_rest::{quick_rest::QuickRestValue, rest_handler};
use meh_http_server_rest::{HttpMiddleware, HttpMidlewareChain};
use slog::{info, o, Drain};

fn main() -> Result<(), TcpError> {
    let num_value = Arc::new(Mutex::new(42));

    let example = async move {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let mut stack = StdTcpStack;

        let addr = meh_http_common::addr::SocketAddrV4::new(
            meh_http_common::addr::Ipv4Addr::new(127, 0, 0, 1),
            8080,
        );
        let listener = stack.create_socket_listener(addr.into()).await?;

        info!(logger, "Listening at http://{}/", addr);

        async fn handle_request(ctx: HttpContext<StdTcpSocket>, num_value: Arc<Mutex<i32>>) {
            let q = {
                let v = QuickRestValue::new_getter_and_setter(
                    "num".into(),
                    {
                        let num_value = num_value.clone();
                        move || {
                            if let Ok(n) = num_value.lock() {
                                *n
                            } else {
                                0
                            }
                        }
                    },
                    move |v| {
                        if let Ok(mut n) = num_value.lock() {
                            *n = v;
                        }
                    },
                );
                quick_rest_value(v)
            };

            let h = HttpMidlewareChain::new(allow_cors_all(), q);
            let h = HttpMidlewareChain::new(h, not_found());

            h.process(ctx).await;
        }

        let num_value = num_value.clone();
        http_server(&logger, listener, |ctx| {
            handle_request(ctx, num_value.clone())
        })
        .await;

        Ok(())
    };

    async_std::task::block_on(example)
}
