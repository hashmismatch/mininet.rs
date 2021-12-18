use std::net::SocketAddr;

use crate::stack::{TcpStack, TcpError, TcpListen, TcpSocket, UdpSocket};
use async_trait::async_trait;
use async_std::{io::{WriteExt, ReadExt}, net::TcpListener};

pub struct StdTcpSocketListener(async_std::net::TcpListener);

#[async_trait]
impl TcpListen for StdTcpSocketListener {
    type TcpSocket = StdTcpSocket;

    async fn accept(&mut self) -> Result<(Self::TcpSocket, crate::addr::SocketAddr), TcpError> {
        match self.0.accept().await {
            Ok((socket, addr)) => {
                let s = StdTcpSocket(socket);
                Ok((s, from_async_socket_addr(addr)))
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
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize, TcpError> {
        self.0.read(buf).await.map_err(|_| TcpError::Unknown)
    }

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

pub fn to_async_socket_addr(addr: crate::addr::SocketAddr) -> async_std::net::SocketAddr {
    match addr {
        embedded_nal::SocketAddr::V4(v4) => {
            async_std::net::SocketAddrV4::new(
                v4.ip().octets().into(),
                v4.port()
            ).into()
        },
        embedded_nal::SocketAddr::V6(v6) => {
            async_std::net::SocketAddrV6::new(v6.ip().octets().into(), v6.port(), v6.flowinfo(), v6.scope_id()).into()
        },
    }
}

pub fn from_async_socket_addr(addr: async_std::net::SocketAddr) -> crate::addr::SocketAddr {
    match addr {
        SocketAddr::V4(v4) => {
            embedded_nal::SocketAddrV4::new(v4.ip().octets().into(), v4.port()).into()
        },
        SocketAddr::V6(v6) => {
            embedded_nal::SocketAddrV6::new(v6.ip().octets().into(), v6.port(), v6.flowinfo(), v6.scope_id()).into()
        },
    }
}

#[async_trait]
impl TcpStack for StdTcpStack {
    type TcpSocket = StdTcpSocket;
    type TcpListener = StdTcpSocketListener;
    type UdpSocket = StdUdpSocket;

    async fn create_socket_connected(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpSocket, TcpError> {

        let addr = to_async_socket_addr(addr);
        let socket = async_std::net::TcpStream::connect(addr).await.map_err(|_e| TcpError::Unknown)?;
        Ok(StdTcpSocket(socket))

    }

    async fn get_socket_address(&self, host_and_port: &str) -> Result<crate::addr::SocketAddr, TcpError> {
        use async_std::net::ToSocketAddrs;

        match host_and_port.to_socket_addrs().await {
            Ok(mut iter) => {
                match iter.next() {
                    Some(a) => {
                        Ok(from_async_socket_addr(a))
                    },
                    _ => Err(TcpError::Unknown)
                }
            },
            Err(_e) => Err(TcpError::Unknown)
        }        
    }

    async fn create_socket_listener(&mut self, addr: crate::addr::SocketAddr) -> Result<Self::TcpListener, TcpError> {
        let listener = TcpListener::bind(to_async_socket_addr(addr)).await.map_err(|_| TcpError::Unknown)?;
        Ok(StdTcpSocketListener(listener))
    }

    async fn create_udp_socket(&mut self) -> Result<Self::UdpSocket, TcpError> {
        let socket = async_std::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|_e| TcpError::Unknown)?;;
        Ok(StdUdpSocket(socket))
    }
}


pub struct StdUdpSocket(async_std::net::UdpSocket);

#[async_trait]
impl UdpSocket for StdUdpSocket {
    async fn read_from<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<(usize, crate::addr::SocketAddr), TcpError> {
        let (len, addr) = self.0.recv_from(buf).await.map_err(|_| TcpError::Unknown)?;
        Ok((len, from_async_socket_addr(addr)))
    }

    async fn send_to(&mut self, addr: crate::addr::SocketAddr, data: &[u8]) -> Result<usize, TcpError> {
        match self.0.send_to(data, to_async_socket_addr(addr)).await {
            Ok(n) => Ok(n),
            Err(_) => Err(TcpError::Unknown)
        }
    }
}