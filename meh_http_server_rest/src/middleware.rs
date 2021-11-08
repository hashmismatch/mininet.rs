use std::{marker::PhantomData, ops::Add, pin::Pin, sync::Arc};

use frunk::{HCons, hlist::Plucker, prelude::HList};
use futures::Future;
use meh_http_common::{resp::HttpStatusCodes, stack::TcpSocket};
use async_trait::async_trait;
use meh_http_server::HttpContext;
use serde_json::json;

use crate::{HandlerResult, HandlerResultOk, extras::Extras, response_builder::HttpResponseBuilder};

#[async_trait]
pub trait HttpMiddlewareRunner: Send + Sized {
    type Context: HttpMiddlewareContext;

    async fn run(self, ctx: HttpResponseBuilder<Self::Context>) -> HandlerResult<Self::Context>;
}


pub trait HttpMiddlewareContext: Send {
    type Socket: TcpSocket;
}


pub struct Ctx<S> {
    pub socket: S
}

impl<S> HttpMiddlewareContext for Ctx<S>
    where S: TcpSocket
{
    type Socket = S;
}


#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Context: HttpMiddlewareContext;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context>;
}




pub struct Null<C> {
    _ctx: PhantomData<C>
}

#[async_trait]
impl<C> HttpMiddlewareRunner for Null<C>
    where C: HttpMiddlewareContext
{
    type Context = C;

    async fn run(self, ctx: HttpResponseBuilder<Self::Context>) -> HandlerResult<Self::Context> {
        Ok(ctx.into())
    }
}

#[async_trait]
impl<C> HttpMiddleware for Null<C>
    where C: HttpMiddlewareContext
{
    type Context = C;

    async fn handle<N>(self, ctx: HttpResponseBuilder<Self::Context>, _next: N) -> HandlerResult<Self::Context>
        where N: HttpMiddlewareRunner<Context = Self::Context> 
    {
        Ok(ctx.into())
    }
}


impl<C> Null<C> {
    pub fn new() -> Self {
        Null { _ctx: PhantomData::default() }
    }
}








// pub struct HttpMidlewareFn<S>
// where
//     S: TcpSocket,
// {
//     func: Box<dyn FnOnce(HttpResponseBuilder<S>) -> HandlerResult<S> + Send>,
// }

// impl<S> HttpMidlewareFn<S>
// where
//     S: TcpSocket,
// {
//     pub fn new<F>(func: F) -> Self
//     where
//         F: Fn(HttpResponseBuilder<S>) -> HandlerResult<S> + Send,
//         F: 'static,
//     {
//         HttpMidlewareFn {
//             func: Box::new(func),
//         }
//     }
// }

// #[async_trait]
// impl<S> HttpMiddleware for HttpMidlewareFn<S>
// where
//     S: TcpSocket,
// {
//     type Socket = S;

//     async fn handle<H, T>(
//         self,
//         mut ctx: HttpResponseBuilder<Self::Socket>,
//         next: NextMiddleware<H, T>
//     ) -> HandlerResult<Self::Socket>    
//     where H: HttpMiddleware<Socket=Self::Socket>,
//     T: NPop + NPop<Target = H> + Send,
//     <T as NPop>::Remainder: Send
//     {
//         todo!();
//         let res = (self.func)(ctx)?;
//         /*
//         match res {
//             HandlerResultOk::Pass(p) => {
//                 //next.process(p).await
//             },
//             _ => Ok(res)
//         }
//         */
//     }
// }


// pub struct HttpMidlewareFnFut<S>
// where
//     S: TcpSocket,
// {
//     func: Box<
//         dyn FnOnce(HttpResponseBuilder<S>) -> Pin<Box<dyn Future<Output = HandlerResult<S>> + Send>>
//             + Send,
//     >,
// }

// impl<S> HttpMidlewareFnFut<S>
// where
//     S: TcpSocket,
// {
//     pub fn new<F, Fut>(func: F) -> Self
//     where
//         F: FnOnce(HttpResponseBuilder<S>) -> Fut,
//         F: Send + 'static,
//         Fut: Future<Output = HandlerResult<S>> + Send + 'static,
//     {
//         Self {
//             func: Box::new(|c| {
//                 let r = func(c);
//                 Box::pin(r)
//             }),
//         }
//     }
// }

// #[async_trait]
// impl<S> HttpMiddleware for HttpMidlewareFnFut<S>
// where
//     S: TcpSocket,
// {
//     type Socket = S;

//     async fn handle<H, T>(
//         self,
//         mut ctx: HttpResponseBuilder<Self::Socket>,
//         next: NextMiddleware<H, T>
//     ) -> HandlerResult<Self::Socket>    
//     where H: HttpMiddleware<Socket=Self::Socket>,
//     T: NPop + NPop<Target = H> + Send,
//     <T as NPop>::Remainder: Send
//     {
//         todo!()
//         /*
//         let res = (self.func)(ctx).await?;
//         match res {
//             HandlerResultOk::Pass(p) => {
//                 next.process(p).await
//             },
//             _ => Ok(res)
//         }
//         */
//     }
// }