use async_trait::async_trait;

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
