use std::{marker::PhantomData, pin::Pin, sync::Arc};

use frunk::{HCons, hlist::Plucker, prelude::HList};
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

/*
fn get_head<H, R>(list: HCons<H, R>) -> (H, R)
    where H: HttpMiddleware
{
    let (head, remainder) = list.pluck();

    (head, remainder)
}
*/



pub struct NextMiddleware<H, T> {
    list: HCons<H, T>
}

impl<H, T> NextMiddleware<H, T>
    where H: HttpMiddleware
{
    pub async fn process<S>(self, ctx: HttpResponseBuilder<S>) -> HandlerResult<S>
        where S: TcpSocket
    {
        todo!()
        /*
        let (first, remainder) = self.list.pluck();
        let next = NextMiddleware {
            list: remainder
        };
        let res = first.handle(ctx, next).await?;
        Ok(res.into())
        */
    }
}
#[derive(Debug)]
pub struct NNil;
pub struct NCons<H, T> {
    pub head: H,
    pub tail: T
}

pub trait NPop {
    type Target;
    type Remainder;

    fn try_get(self) -> Option<(Self::Target, Self::Remainder)>;
}

impl<H, T> NPop for NCons<H, T> {
    type Target = H;
    type Remainder = T;

    fn try_get(self) -> Option<(Self::Target, Self::Remainder)> {
        Some((
            self.head,
            self.tail
        ))
    }
}

impl NPop for NNil {
    type Target = NNil;
    type Remainder = NNil;

    fn try_get(self) -> Option<(Self::Target, Self::Remainder)> {
        None
    }
}

pub struct NextMiddleware2<H, T> {
    list: NCons<H, T>
}

impl<H, T> NextMiddleware2<H, T>
    where H: core::fmt::Debug, T: NPop
    {
    pub fn next(self) -> Option<NextMiddleware2<<T as NPop>::Target, <T as NPop>::Remainder>>
    {
        if let Some((head, tail)) = self.list.try_get() {
            println!("value: {:?}", head);

            let p2 = tail.try_get();
            if let Some((h2, t2)) = p2 {
                return Some(
                    NextMiddleware2 {
                        list: NCons {
                            head: h2,
                            tail: t2
                        }
                    }
                );
            }            
        }
        
        None
    }
}

#[cfg(test)]
#[test]
fn plucky() {
    use frunk::hlist;

    /*
    use crate::{error_handler::error_handler, helpers::allow_cors_all};

    let chain = hlist![
        allow_cors_all(),
        error_handler()
    ];
    */

    /*
    let s = hlist![
        "123",
        123,
        123.0
    ];
    */

    let n = NCons {
        head: 123,
        tail: NCons {
            head: "123",
            tail: NCons {
                head: 123.0,
                tail: NNil
            }
        }
    };

    let mw = NextMiddleware2 { list: n };
    let mw = mw.next().unwrap();
    let mw = mw.next().unwrap();
    let mw = mw.next();
    assert!(mw.is_none());

    //let (h, t) = n.try_get().unwrap();
    //let (h, t) = t.try_get().unwrap();
    
}



#[async_trait]
pub trait HttpMiddleware: Send + Sized {
    type Socket: TcpSocket;

    async fn handle<H, T>(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
        next: NextMiddleware<H, T>
    ) -> HandlerResult<Self::Socket>    
    where H: HttpMiddleware<Socket=Self::Socket>, T: Send;
}






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

    async fn handle<H, T>(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
        next: NextMiddleware<H, T>
    ) -> HandlerResult<Self::Socket>    
    where H: HttpMiddleware<Socket=Self::Socket>, T: Send
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

    async fn handle<H, T>(
        self,
        mut ctx: HttpResponseBuilder<Self::Socket>,
        next: NextMiddleware<H, T>
    ) -> HandlerResult<Self::Socket>    
    where H: HttpMiddleware<Socket=Self::Socket>, T: Send
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