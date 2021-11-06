use std::{fmt::Debug, marker::PhantomData};

use meh_http_common::stack::TcpSocket;

use crate::{HandlerResult, response_builder::HttpResponseBuilder};

/*
pub enum List<T> where T: Debug {
    Entry(T, Box<T>),
    End
}

impl<T> List<T> where T: Debug {
    pub fn apply()
}
*/


pub trait HttpMid: Sized {
    type Socket: TcpSocket;

    fn handle<N>(
        self,
        ctx: HttpResponseBuilder<Self::Socket>,
        next: N
    ) -> HandlerResult<Self::Socket>
        where N: HttpMidInner<Socket=Self::Socket>;

    fn chain<B>(self, other: B) -> HttpWrap<Self::Socket, Self, B>
        where B: HttpMid
    {
        HttpWrap {
            _socket: PhantomData::default(),
            a: self,
            b: other
        }
    }
}

pub trait HttpMidInner : Sized
{
    type Socket: TcpSocket;

    fn run(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket>;

    fn chain<B>(self, other: B) -> HttpWrap<Self::Socket, Self, B>
        where B: HttpMid
    {
        HttpWrap {
            _socket: PhantomData::default(),
            a: self,
            b: other
        }
    }    
}



pub struct SampleMid<S> {
    _socket: PhantomData<S>
}

impl<S> HttpMid for SampleMid<S>
    where S: TcpSocket
{
    type Socket = S;

    fn handle<N>(
        self,
        ctx: HttpResponseBuilder<Self::Socket>,
        next: N
    ) -> HandlerResult<Self::Socket>
        where N: HttpMidInner<Socket=Self::Socket>
     {
        next.run(ctx)
    }
}

pub struct HttpMidInnerCons<S> {
    _socket: PhantomData<S>
}

impl<S> HttpMidInner for HttpMidInnerCons<S>
    where S: TcpSocket
{
    type Socket = S;

    fn run(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        Ok(ctx.into())
    }
}

pub struct HttpWrap<S, A, B> {
    a: A,
    b: B,
    _socket: PhantomData<S>
}



impl<S, A, B> HttpMidInner for HttpWrap<S, A, B>
    where S: TcpSocket,
    A: HttpMid<Socket=S>,
    B: HttpMid<Socket=S>
{
    type Socket = S;

    fn run(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        self.a.handle(ctx, HttpWrapNull { a: self.b, _socket: PhantomData::default() })
    }
}

pub struct HttpWrapNull<S, A>
{
    a: A,
    _socket: PhantomData<S>
}

impl<S, A> HttpMidInner for HttpWrapNull<S, A>
    where S: TcpSocket,
    A: HttpMid<Socket=S>
{
    type Socket=S;

    fn run(self, ctx: HttpResponseBuilder<Self::Socket>) -> HandlerResult<Self::Socket> {
        self.a.handle(ctx, HttpMidInnerCons { _socket: Default::default() })
    }
}

