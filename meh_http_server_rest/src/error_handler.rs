use std::marker::PhantomData;
use crate::{HandlerResult, middleware::{HttpMiddleware, NextMiddleware}, response_builder::HttpResponseBuilder};
use async_trait::async_trait;
use meh_http_common::stack::TcpSocket;
use slog::{debug, error};

pub fn error_handler<S>() -> ErrorHandler<S> {
    ErrorHandler {
        _socket: Default::default()
    }
}

pub struct ErrorHandler<S> {
    _socket: PhantomData<S>
}

#[async_trait]
impl<S> HttpMiddleware for ErrorHandler<S>
    where S: TcpSocket
{
    type Socket=S;

    async fn handle<H, T>(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
        next: NextMiddleware<H, T>
    ) -> HandlerResult<Self::Socket>    
    where H: HttpMiddleware<Socket=Self::Socket>, T: Send   
    {
        let logger = ctx.logger.clone();
        debug!(logger, "Error handler start.");

        let res = next.process(ctx).await;
        match res {
            Ok(_) => (),
            Err(ref e) => {
                error!(logger, "Encountered an error: {:?}", e);
            }
        }

        res
    }
}