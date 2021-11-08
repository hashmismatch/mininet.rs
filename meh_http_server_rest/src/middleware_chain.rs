use std::marker::PhantomData;
use std::ops::Add;

use crate::HandlerResult;
use crate::middleware::HttpMiddlewareContext;
use crate::middleware::HttpMiddlewareRunner;
use crate::middleware::HttpMiddleware;
use crate::middleware::Null;
use crate::response_builder::HttpResponseBuilder;
use async_trait::async_trait;

pub struct Chain<C, H, T>
    where C: HttpMiddlewareContext,
        H: HttpMiddleware,
        T: HttpMiddlewareRunner
{
    _ctx: PhantomData<C>,
    head: H,
    tail: T
}

#[async_trait]
impl<C, H, T> HttpMiddlewareRunner for Chain<C, H, T>
where C: HttpMiddlewareContext,
        H: HttpMiddleware<Context = C>,
        T: HttpMiddlewareRunner<Context = C>
{
    type Context = C;

    async fn run(self, ctx: HttpResponseBuilder<Self::Context>) -> HandlerResult<Self::Context> {
        self.head.handle(ctx, self.tail).await
    }
}

impl<C, H, T> Chain<C, H, T>
where C: HttpMiddlewareContext,
        H: HttpMiddleware,
        T: HttpMiddlewareRunner
{
    pub fn chain<N>(self, with: N) -> <Self as Add<Chain<C, N, Null<C>>>>::Output
        where
            N: HttpMiddleware,
            Self: Add<Chain<C, N, Null<C>>>
    {
        let with = Chain::new(with);
        self + with
    }
}

impl<C, H> Chain<C, H, Null<C>>
where C: HttpMiddlewareContext,
        H: HttpMiddleware
{
    pub fn new(middleware: H) -> Self {
        Self {
            _ctx: PhantomData::default(),
            head: middleware,
            tail: Null::new()
        }
    }
}

pub trait ChainElement { }

impl<C, H, T> ChainElement for Chain<C, H, T>
where C: HttpMiddlewareContext,
        H: HttpMiddleware,
        T: HttpMiddlewareRunner
{
    
}

impl<C> ChainElement for Null<C> {

}

impl<C, RHS> Add<RHS> for Null<C>
where
    RHS: ChainElement
{
    type Output = RHS;

    fn add(self, rhs: RHS) -> RHS {
        rhs
    }
}


impl<C, H, T, RHS> Add<RHS> for Chain<C, H, T>
where C: HttpMiddlewareContext,
        H: HttpMiddleware,
        T: HttpMiddlewareRunner + Add<RHS>,
        <T as Add<RHS>>::Output: HttpMiddlewareRunner,
{
    type Output = Chain<C, H, <T as Add<RHS>>::Output>;

    fn add(self, rhs: RHS) -> Self::Output {
        Chain {
            _ctx: PhantomData::default(),
            head: self.head,
            tail: self.tail + rhs
        }
    }
}

