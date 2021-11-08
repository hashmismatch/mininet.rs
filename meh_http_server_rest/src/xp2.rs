use core::fmt::Debug;
use std::{marker::PhantomData, ops::Add};

use crate::middleware::NCons;


pub trait HttpMidNext: Sized {
    type Ctx: Debug;

    fn run(self, ctx: Self::Ctx) -> ();
}

pub trait HttpMid: Sized {
    type Ctx: Debug;

    fn handle<H>(
        self,
        ctx: Self::Ctx,
        next: H
    ) -> ()
    where 
        H: HttpMidNext<Ctx=Self::Ctx>;
}

pub struct HttpMidNull<C> {
    _ctx: PhantomData<C>
}

impl<C> ChainElement for HttpMidNull<C> {
    
}

impl<C> HttpMidNull<C> {
    pub fn new() -> Self {
        Self {
            _ctx: PhantomData::default()
        }
    }
}

impl<C> HttpMid for HttpMidNull<C>
    where C: Debug
{
    type Ctx = C;

    fn handle<H>(
        self,
        ctx: Self::Ctx,
        next: H
    ) -> ()
    where 
        H: HttpMidNext<Ctx=Self::Ctx>
    {
        ()
    }
}

impl<C> HttpMidNext for HttpMidNull<C>
    where C: Debug
{
    type Ctx = C;

    fn run(self, ctx: Self::Ctx) -> () {
        ()
    }
}


pub struct PrintIt<C> {
    _ctx: PhantomData<C>,
    num: usize
}

impl<C> HttpMid for PrintIt<C>
    where C: Debug
{
    type Ctx = C;

    fn handle<H>(
        self,
        ctx: Self::Ctx,
        next: H
    ) -> ()
    where 
        H: HttpMidNext<Ctx=Self::Ctx>
    {        
        println!("ctx val: {:?}, my num: {}", ctx, self.num);

        next.run(ctx);
    }
}


#[derive(Debug)]
struct SomeContext {
    foo: usize
}


struct Chain<C, A, B>
    where C: Debug,
    A: HttpMid,
    B: HttpMidNext
{
    _ctx: PhantomData<C>,
    a: A,
    b: B
}

impl<C, A, B> Chain<C, A, B>
where C: Debug,
    A: HttpMid,
    B: HttpMidNext
{
    pub fn chain<N>(self, with: N) -> <Self as Add<Chain<C, N, HttpMidNull<C>>>>::Output
        where
            N: HttpMid,
            Self: Add<Chain<C, N, HttpMidNull<C>>>
    {
        let with = Chain::new(with);
        self + with
    }
}

impl<C, A, B> ChainElement for Chain<C, A, B>
where C: Debug,
    A: HttpMid,
    B: HttpMidNext
{
    
}

impl<C, A> Chain<C, A, HttpMidNull<C>>
where C: Debug,
    A: HttpMid
{
    pub fn new(a: A) -> Self {
        Self {
            _ctx: PhantomData::default(),
            a,
            b: HttpMidNull::new()
        }
    }
}

impl<C, A, B> HttpMidNext for Chain<C, A, B>
    where C: Debug,
    A: HttpMid<Ctx=C>,
    B: HttpMidNext<Ctx=C>
{
    type Ctx = C;

    fn run(self, ctx: Self::Ctx) -> () {
        self.a.handle(ctx, self.b)
    }
}


pub trait ChainElement { }


impl<C, RHS> Add<RHS> for HttpMidNull<C>
where
    RHS: ChainElement
{
    type Output = RHS;

    fn add(self, rhs: RHS) -> RHS {
        rhs
    }
}

impl<C, A, B, RHS> Add<RHS> for Chain<C, A, B>
where
    C: Debug,
    A: HttpMid,
    B: HttpMidNext,
    B: Add<RHS>,
    <B as Add<RHS>>::Output: HttpMidNext,
    RHS: ChainElement
{
    type Output = Chain<C, A, <B as Add<RHS>>::Output>;

    fn add(self, rhs: RHS) -> Self::Output {
        Chain {
            _ctx: PhantomData::default(),
            a: self.a,
            b: self.b + rhs
        }
    }
}



#[test]
fn test_hm() {

    let ctx = SomeContext { foo: 100 };
    
    let a = PrintIt { _ctx: PhantomData::default(), num: 1};
    let b = PrintIt { _ctx: PhantomData::default(), num: 2};
    let c = PrintIt { _ctx: PhantomData::default(), num: 3};

    /*
    let chain3 = Chain {
        _ctx: PhantomData::default(),
        a: c,
        b: HttpMidNull::new()
    };
    
    let chain2 = Chain {
        _ctx: PhantomData::default(),
        a: b,
        b: chain3
    };

    let chain = Chain {
        _ctx: PhantomData::default(),
        a: a,
        b: chain2
    };
    */

    //let chain = Chain::new(a) + Chain::new(b) + Chain::new(c);
    let chain = Chain::new(a)
    .chain(b)
    .chain(c);

    chain.run(ctx);
}
