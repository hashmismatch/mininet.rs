use mininet_base::{stack::{TcpStack, TcpError, UdpSocket}, addr::SocketAddr};

use crate::proto::{SntpData, NtpEpochTime};


#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct SntpTimeOffset {
    milliseconds: i64
}

pub async fn get_sntp_time_offset<S, F>(stack: &mut S, sntp_server: SocketAddr, get_current_time: F) -> Result<SntpTimeOffset, TcpError>
    where F: Fn() -> NtpEpochTime, S: TcpStack
{    
    let mut socket = stack.create_udp_socket().await?;

    let now = get_current_time();
    let req = SntpData::new_request_sec(now);

    let resp = socket.send_to(sntp_server, &req.get_data()).await?;
    
    let mut buf = [0; 48];
    let (recv, address) = socket.read_from(&mut buf).await?;
    if recv != 48 {
        return Err(TcpError::Unknown);
    }

    let received_at = get_current_time();;

    let sntp_resp = SntpData::from_buffer(&buf).map_err(|_| TcpError::Unknown)?;
    let offset = sntp_resp.local_time_offset(received_at);

    let offset = SntpTimeOffset {
        milliseconds: offset
    };

    Ok(offset)
}