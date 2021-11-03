use std::{future::Future, time::Duration};

use async_io::Timer;
use futures::FutureExt;
use meh_http_common::stack::SystemEnvironment;



#[derive(Copy, Clone, Debug)]
pub struct StdEnv;

impl SystemEnvironment for StdEnv {
    type Timeout = EspTimeout;

    fn timeout(&self, timeout: Duration) -> EspTimeout {
        EspTimeout(Timer::after(timeout))
    }
}

pub struct EspTimeout(Timer);

impl Future for EspTimeout {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map(|_| ())
    }
}
