use crate::stack::{TcpError, TcpListen, TcpSocket};
use async_trait::async_trait;
use async_std::io::{WriteExt, ReadExt};

pub struct StdTcpSocketListener(async_std::net::TcpListener);

#[async_trait]
impl TcpListen for StdTcpSocketListener {
    type TcpSocket = StdTcpSocket;

    async fn accept(&mut self) -> Result<Self::TcpSocket, TcpError> {
        match self.0.accept().await {
            Ok((socket, _addr)) => {
                let s = StdTcpSocket(socket);
                Ok(s)
            },
            Err(_) => {
                Err(TcpError::Unknown)
            }
        }
    }
}

pub struct StdTcpSocket(async_std::net::TcpStream);

#[async_trait]
impl TcpSocket for StdTcpSocket {
    async fn read_to_end(&mut self) -> Result<Vec<u8>, TcpError> {
        let mut buf = vec![];
        match self.0.read_to_end(&mut buf).await {
            Ok(s) => Ok(buf),
            Err(_) => Err(TcpError::Unknown)
        }
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize, TcpError> {
        match self.0.write(data).await {
            Ok(n) => Ok(n),
            Err(_) => Err(TcpError::Unknown)
        }
    }
}