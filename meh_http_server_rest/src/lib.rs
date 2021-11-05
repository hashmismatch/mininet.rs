pub mod extras;
pub mod helpers;
pub mod middleware;
pub mod openapi;
pub mod quick_rest;
pub mod response_builder;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;

use meh_http_common::req::HttpServerHeader;
use meh_http_common::resp::{HttpResponseWriter, HttpStatusCodes};
use meh_http_common::stack::{TcpError, TcpSocket};
use meh_http_server::HttpContext;
use response_builder::{HttpReponseComplete, HttpResponseBuilder};
use slog::warn;

#[derive(Debug)]
pub enum RestError {
    TcpError(TcpError),
    Unknown,
}

impl From<TcpError> for RestError {
    fn from(v: TcpError) -> Self {
        Self::TcpError(v)
    }
}

pub type HandlerResult<S> = Result<HandlerResultOk<S>, RestError>;

pub enum HandlerResultOk<S>
where
    S: TcpSocket,
{
    Complete(HttpReponseComplete),
    Pass(HttpResponseBuilder<S>),
}

impl<S> From<HttpReponseComplete> for HandlerResultOk<S>
where
    S: TcpSocket,
{
    fn from(v: HttpReponseComplete) -> Self {
        Self::Complete(v)
    }
}

impl<S> From<HttpResponseBuilder<S>> for HandlerResultOk<S>
where
    S: TcpSocket,
{
    fn from(v: HttpResponseBuilder<S>) -> Self {
        Self::Pass(v)
    }
}
