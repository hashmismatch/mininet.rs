#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod extras;
pub mod helpers;
pub mod middleware;
pub mod middleware_chain;
pub mod middleware_fn;
pub mod openapi;
pub mod response_builder;
pub mod error_handler;
pub mod quick_rest;

use mininet_base::stack::{TcpError};
use middleware::HttpMiddlewareContext;
use response_builder::{HttpReponseComplete, HttpResponseBuilder};

pub type RestResult<T = ()> = Result<T, RestError>;

use alloc::borrow::Cow;

#[derive(Debug)]
pub enum RestError {
    TcpError(TcpError),
    Unknown,
    ErrorMessage(Cow<'static, str>)
}

pub struct RestErrorContext<S>
    where S: HttpMiddlewareContext
{
    pub ctx: Option<HttpResponseBuilder<S>>,
    pub error: RestError
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


pub type HandlerResult<S> = Result<HandlerResultOk<S>, RestErrorContext<S>>;

pub struct HandlerError<S>
    where S: HttpMiddlewareContext
{
    pub error: RestError,
    pub ctx: HttpResponseBuilder<S>
}

pub enum HandlerResultOk<S>
where
    S: HttpMiddlewareContext,
{
    Complete(HttpReponseComplete),
    Pass(HttpResponseBuilder<S>),
}

impl<S> From<HttpReponseComplete> for HandlerResultOk<S>
where
    S: HttpMiddlewareContext,
{
    fn from(v: HttpReponseComplete) -> Self {
        Self::Complete(v)
    }
}

impl<S> From<HttpResponseBuilder<S>> for HandlerResultOk<S>
where
    S: HttpMiddlewareContext,
{
    fn from(v: HttpResponseBuilder<S>) -> Self {
        Self::Pass(v)
    }
}
