pub mod extras;
pub mod helpers;
pub mod middleware;
pub mod openapi;
pub mod quick_rest;
pub mod response_builder;
pub mod error_handler;
pub mod xp;

use std::any::{Any, TypeId};
use std::borrow::Cow;
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

pub type RestResult<T = ()> = Result<T, RestError>;

#[derive(Debug)]
pub enum RestError {
    TcpError(TcpError),
    Unknown,
    ErrorMessage(Cow<'static, str>)
}

impl From<serde_json::Error> for RestError {
    fn from(e: serde_json::Error) -> Self {
        Self::ErrorMessage(format!("JSON error: {}", e).into())
    }
}

impl From<TcpError> for RestError {
    fn from(v: TcpError) -> Self {
        Self::TcpError(v)
    }
}

pub type HandlerResult<S> = Result<HandlerResultOk<S>, RestError>;

pub struct HandlerError<S>
    where S: TcpSocket
{
    pub error: RestError,
    pub ctx: HttpResponseBuilder<S>
}

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
