use core::time::Duration;
use alloc::boxed::Box;
use alloc::vec::Vec;
use async_trait::async_trait;

#[derive(Debug, Copy, Clone)]
pub enum TcpError {
    Closed,
    Timeout,
    Unknown
}


#[async_trait]
pub trait TcpListen {
    type TcpSocket: TcpSocket;

    async fn accept(&mut self) -> Result<(Self::TcpSocket, crate::addr::SocketAddr), TcpError>;
}

#[async_trait]
pub trait TcpSocket: Send + Sync + Sized + 'static {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize, TcpError>;
    async fn read_to_end(&mut self) -> Result<Vec<u8>, TcpError>;
    async fn send(&mut self, data: &[u8]) -> Result<usize, TcpError>;
}

#[async_trait]
pub trait TcpStack {
    type TcpSocket: TcpSocket;
    type TcpListener: TcpListen;

    async fn create_socket_listener(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpListener, TcpError>;
    async fn create_socket_connected(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpSocket, TcpError>;
    async fn get_socket_address(&self, host_and_port: &str) -> Result<crate::addr::SocketAddr, TcpError>;
}



pub trait SystemEnvironment: Clone {
    type Timeout: core::future::Future<Output=()> + Unpin + Send;
    
    fn timeout(&self, timeout: Duration) -> Self::Timeout;
}


pub async fn with_timeout<E, Fut, FutOut>(env: &E, future: Fut, timeout: Duration) -> Result<FutOut, TcpError>
    where 
        E: SystemEnvironment,
        Fut: core::future::Future<Output=FutOut>
{
    let timeout = env.timeout(timeout);
    let future = Box::pin(future);
    match futures::future::select(future, timeout).await {
        futures::future::Either::Left((f, _)) => Ok(f),
        futures::future::Either::Right(_) => Err(TcpError::Timeout)
    }
}
