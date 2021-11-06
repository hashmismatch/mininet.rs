use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use meh_http_common::stack::TcpError;
use meh_http_common::stack::TcpStack;
use meh_http_common::std::StdTcpSocket;
use meh_http_common::std::StdTcpStack;
use meh_http_server::http_server;
use meh_http_server::HttpContext;
use meh_http_server_rest::RestError;
use meh_http_server_rest::helpers::allow_cors_all;
use meh_http_server_rest::helpers::not_found;
use meh_http_server_rest::middleware::HttpMiddleware;
use meh_http_server_rest::openapi::Info;
use meh_http_server_rest::openapi::Server;
use meh_http_server_rest::quick_rest::enable_open_api;
use meh_http_server_rest::quick_rest::openapi_final_handler;
use meh_http_server_rest::quick_rest::quick_rest_value_with_openapi;
use meh_http_server_rest::{quick_rest::QuickRestValue};
use meh_std_tests::StdEnv;
use slog::{info, o, Drain};

fn main() -> Result<(), TcpError> {
    let num_value = Arc::new(Mutex::new(42));
    let str_value = Arc::new(Mutex::new("foobar".to_string()));

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

        async fn handle_request(ctx: HttpContext<StdTcpSocket>, num_value: Arc<Mutex<usize>>, str_value: Arc<Mutex<String>>) {
            let api_id = "/simple";

            let q = {
                let v = QuickRestValue::new_getter_and_setter(
                    api_id.into(),
                    "num".into(),
                    {
                        let num_value = num_value.clone();
                        move || {
                            if let Ok(n) = num_value.lock() {
                                Ok(*n)
                            } else {
                                Err(RestError::ErrorMessage("Failed to lock the value.".into()))
                            }
                        }
                    },
                    move |v| {
                        if let Ok(mut n) = num_value.lock() {
                            *n = v;
                            Ok(())
                        } else {
                            Err(RestError::ErrorMessage("Failed to lock the value.".into()))
                        }
                    },
                );
                quick_rest_value_with_openapi(v)
            };

            let q2 = {
                let v = QuickRestValue::new_getter_and_setter(
                    api_id.into(),
                    "str".into(),
                    {
                        let str_value = str_value.clone();
                        move || {
                            if let Ok(s) = str_value.lock() {
                                Ok(s.clone())
                            } else {
                                Err(RestError::ErrorMessage("Failed to lock the value.".into()))
                            }
                        }
                    },
                    move |v| {
                        if let Ok(mut s) = str_value.lock() {
                            *s = v;
                            Ok(())
                        } else {
                            Err(RestError::ErrorMessage("Failed to lock the value.".into()))
                        }
                    },
                );
                quick_rest_value_with_openapi(v)
            };

            let h = allow_cors_all()
                .chain(enable_open_api(Info { title: "API".into(), description: "yay".into(), version: "0.1.0".into() }, vec![
                    Server {
                        url: "http://localhost:8080".into(),
                        description: "dev".into()
                    }
                ]))
                .chain(q)
                .chain(q2)
                .chain(openapi_final_handler())
                .chain(not_found());
                
            h.process(ctx).await;
        }

        let env = StdEnv;

        let num_value = num_value.clone();
        http_server(&logger, env, listener, |ctx| {
            handle_request(ctx, num_value.clone(), str_value.clone())
        }, Some(Duration::from_secs(10))).await;

        Ok(())
    };

    async_std::task::block_on(example)
}
