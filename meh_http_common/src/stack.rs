use async_trait::async_trait;
use embedded_nal::IpAddr;

#[derive(Debug, Copy, Clone)]
pub enum TcpError {
    Closed,
    Unknown
}

#[async_trait]
pub trait TcpListen {
    type TcpSocket: TcpSocket;

    async fn accept(&mut self) -> Result<Self::TcpSocket, TcpError>;
}

#[async_trait]
pub trait TcpSocket {
    async fn read_to_end(&mut self) -> Result<Vec<u8>, TcpError>;
    async fn send(&mut self, data: &[u8]) -> Result<usize, TcpError>;
}

#[async_trait]
pub trait TcpStack {
    type TcpSocket: TcpSocket;

    async fn create_socket_connected(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpSocket, TcpError>;
    async fn get_socket_address(&self, host_and_port: &str) -> Result<crate::addr::SocketAddr, TcpError>;
}