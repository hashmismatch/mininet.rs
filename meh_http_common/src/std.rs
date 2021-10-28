use std::net::SocketAddr;

use crate::stack::{TcpStack, TcpError, TcpListen, TcpSocket};
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

#[derive(Default)]
pub struct StdTcpStack;

#[async_trait]
impl TcpStack for StdTcpStack {
    type TcpSocket = StdTcpSocket;

    async fn create_socket_connected(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpSocket, TcpError> {
        panic!("todo");
    }

    async fn get_socket_address(&self, host_and_port: &str) -> Result<crate::addr::SocketAddr, TcpError> {
        use async_std::net::ToSocketAddrs;

        match host_and_port.to_socket_addrs().await {
            Ok(mut iter) => {
                match iter.next() {
                    Some(SocketAddr::V4(v4)) => {
                        let net_ip = v4.ip().octets();
                        let ip = crate::addr::Ipv4Addr::new(net_ip[0], net_ip[1], net_ip[2], net_ip[3]);
                        let s = crate::addr::SocketAddrV4::new(ip, v4.port());
                        Ok(s.into())
                    },
                    _ => Err(TcpError::Unknown)
                }
            },
            Err(_e) => Err(TcpError::Unknown)
        }        
    }
}