use std::{marker::PhantomData, pin::Pin, sync::Arc};

use frunk::HCons;
use futures::Future;
use meh_http_common::{resp::HttpStatusCodes, stack::TcpSocket};
use async_trait::async_trait;
use meh_http_server::HttpContext;
use serde_json::json;

use crate::{HandlerResult, HandlerResultOk, extras::Extras, response_builder::HttpResponseBuilder};


/*
pub struct Next<'a, S> where S: TcpSocket {
    pub(crate) next_middleware: &'a [Arc<dyn HttpMiddleware<Socket=S>>]
}
*/

pub struct ChainTail<A, B, S> {
    pub head: A,
    pub tail: B,
    _socket: PhantomData<S>
}

fn get_head<H, R>(list: HCons<H, R>) -> (H, R)
    where H: HttpMiddleware
{
    let (head, remainder) = list.pluck();

    (head, remainder)
}



/*
impl<A, S> ChainTail<A, HttpMiddlewareNull<S>, S> {

}
*/





/*
pub fn new_chain<S, A>(start: A) -> ChainTail<A, HttpMiddlewareNull<S>, S> {
    ChainTail {
        head: start,
        tail: HttpMiddlewareNull { _socket: Default::default() },
        _socket: Default::default()
    }
}



#[async_trait]
impl<A, B, S> HttpMiddlewareNext for ChainTail<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddlewareNext<Socket=S>,
        S: TcpSocket
{
    type Socket=S;
    
    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        self.a.handle(ctx, self.b).await
    }
}
*/




#[async_trait]
pub trait HttpMiddlewareNext: Send + Sized {
    type Socket: TcpSocket;

    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket>;

    /*
    fn http_chain<B>(self, other: B) -> HttpMidlewareChain<Self, HttpMiddlewareWrap<Self::Socket, B>, Self::Socket>
    where
        B: HttpMiddleware<Socket = Self::Socket>,
    {
        let w = HttpMiddlewareWrap {
            _socket: PhantomData::default(),
            a: other
        };
        HttpMidlewareChain {
            a: self,
            b: w,
            _socket: Default::default()
        }
    }
    */

    async fn run(self, ctx: HttpContext<Self::Socket>) -> HandlerResult<Self::Socket>
    {
        let resp_builder = HttpResponseBuilder {
            additional_headers: vec![],
            ctx,
            extras: Extras::default(),
        };

        let res = self.process(resp_builder).await;

        res
    }    
}




#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Socket: TcpSocket;

    async fn handle<N>(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
        next: N
    ) -> HandlerResult<Self::Socket>    
    where N: HttpMiddlewareNext<Socket=Self::Socket>;

    fn http_chain<B>(self, other: B) -> HttpMidlewareChainSecond<Self, B, Self::Socket>
    where
        B: HttpMiddleware<Socket = Self::Socket>,
    {
        HttpMidlewareChainSecond {
            a: self,
            b: other,
            _socket: Default::default()
        }
    }
}


pub struct HttpMidlewareChainFirst<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddlewareNext<Socket=S>,
        S: TcpSocket
{
    a: A,
    b: B,
    _socket: PhantomData<S>,
}

#[async_trait]
impl<A, B, S> HttpMiddlewareNext for HttpMidlewareChainFirst<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddlewareNext<Socket=S>,
        S: TcpSocket
{
    type Socket=S;
    
    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        self.a.handle(ctx, self.b).await
    }
}


pub struct HttpMidlewareChainSecond<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    a: A,
    b: B,
    _socket: PhantomData<S>,
}

impl<A, B, S> HttpMidlewareChainSecond<A, B, S>
where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    pub fn http_chain<O>(self, other: O) -> HttpMidlewareChainFirst<O, Self, S>
    where
        O: HttpMiddleware<Socket = S>
    {
        // todo: not quite ok, it sets "other" as the first one
        HttpMidlewareChainFirst {
            a: other,
            b: self,
            _socket: Default::default()
        }
    }
}

#[async_trait]
impl<A, B, S> HttpMiddlewareNext for HttpMidlewareChainSecond<A, B, S>
    where 
        A: HttpMiddleware<Socket=S>,
        B: HttpMiddleware<Socket=S>,
        S: TcpSocket
{
    type Socket=S;
    
    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        //self.a.process(ctx);
        //self.a.handle(ctx, self.b).await
        //self.a.handle(ctx, HttpMiddlewareWrap { a: self.b, _socket: Default::default ()}).await
        self.a.handle(ctx, HttpMiddlewareWrap { a: self.b, _socket: Default::default ()}).await
    }
}


pub struct HttpMiddlewareWrap<S, A> {
    _socket: PhantomData<S>,
    a: A
}

#[async_trait]
impl<S, A> HttpMiddlewareNext for HttpMiddlewareWrap<S, A>
    where S: TcpSocket, A: HttpMiddleware<Socket=S>
{
    type Socket=S;
    
    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        self.a.handle(ctx, HttpMiddlewareNull { _socket: Default::default() }).await
    }
}

#[async_trait]
impl<S, A> HttpMiddleware for HttpMiddlewareWrap<S, A>
    where S: TcpSocket, A: HttpMiddleware<Socket=S>
{
    type Socket=S;
    
    async fn handle<N>(
        self,
        ctx: HttpResponseBuilder<Self::Socket>,
        next: N
    ) -> HandlerResult<Self::Socket>    
    where N: HttpMiddlewareNext<Socket=Self::Socket> {
        self.a.handle(ctx, next).await
    }
}

/*
pub struct HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    a: A,
    b: B,
    _socket: PhantomData<S>,
}

impl<A, B, S> HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    pub fn new_pair(a: A, b: B) -> Self {
        HttpMidlewareChain {
            a,
            b,
            _socket: PhantomData::default(),
        }
    }

    pub fn chain<C>(self, c: C) -> HttpMidlewareChain<Self, C, S>
    where
        C: HttpMiddleware<Socket = S>,
    {
        HttpMidlewareChain {
            a: self,
            b: c,
            _socket: PhantomData::default(),
        }
    }
}
*/


#[derive(Default)]
pub struct HttpMiddlewareNull<S> {
    _socket: PhantomData<S>
}

#[async_trait]
impl<S> HttpMiddleware for HttpMiddlewareNull<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle<N>(self, mut ctx: HttpResponseBuilder<Self::Socket>, next: N) -> HandlerResult<Self::Socket>    
        where N: HttpMiddlewareNext<Socket=Self::Socket>
    {
        Ok(ctx.into())
    }
}

#[async_trait]
impl<S> HttpMiddlewareNext for HttpMiddlewareNull<S>
where
    S: TcpSocket
{
    type Socket = S;

    async fn process(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        Ok(ctx.into())
    }    
}


/*
#[async_trait]
impl<A, B, S> HttpMiddleware for HttpMidlewareChain<A, B, S>
where
    A: HttpMiddleware<Socket = S>,
    B: HttpMiddleware<Socket = S>,
    S: TcpSocket,
{
    type Socket = S;

    async fn handle(self, ctx: HttpResponseBuilder<S>) -> HandlerResult<S> {
        let res_a = self.a.handle(ctx).await?;
        match res_a {
            HandlerResultOk::Pass(ctx) => self.b.handle(ctx).await,
            _ => Ok(res_a),
        }
    }
}
*/

pub struct HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    func: Box<dyn FnOnce(HttpResponseBuilder<S>) -> HandlerResult<S> + Send>,
}

impl<S> HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(HttpResponseBuilder<S>) -> HandlerResult<S> + Send,
        F: 'static,
    {
        HttpMidlewareFn {
            func: Box::new(func),
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFn<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle<N>(self, mut ctx: HttpResponseBuilder<Self::Socket>, next: N) -> HandlerResult<Self::Socket>    
        where N: HttpMiddlewareNext<Socket=Self::Socket> 
    {
        let res = (self.func)(ctx)?;
        match res {
            HandlerResultOk::Pass(p) => {
                next.process(p).await
            },
            _ => Ok(res)
        }
    }
}


pub struct HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    func: Box<
        dyn FnOnce(HttpResponseBuilder<S>) -> Pin<Box<dyn Future<Output = HandlerResult<S>> + Send>>
            + Send,
    >,
}

impl<S> HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: FnOnce(HttpResponseBuilder<S>) -> Fut,
        F: Send + 'static,
        Fut: Future<Output = HandlerResult<S>> + Send + 'static,
    {
        Self {
            func: Box::new(|c| {
                let r = func(c);
                Box::pin(r)
            }),
        }
    }
}

#[async_trait]
impl<S> HttpMiddleware for HttpMidlewareFnFut<S>
where
    S: TcpSocket,
{
    type Socket = S;

    async fn handle<N>(self, mut ctx: HttpResponseBuilder<Self::Socket>, next: N) -> HandlerResult<Self::Socket>    
        where N: HttpMiddlewareNext<Socket=Self::Socket>
    {
        let res = (self.func)(ctx).await?;
        match res {
            HandlerResultOk::Pass(p) => {
                next.process(p).await
            },
            _ => Ok(res)
        }
    }
}