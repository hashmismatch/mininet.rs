use std::time::Duration;

use mininet_base::std::StdTcpStack;
use mininet_base::stack::TcpStack;
use mininet_base::stack::TcpError;
use mininet_sntp_client::proto::NtpEpochTime;
use mininet_std_tests::StdEnv;
use slog::{o, Drain, info};
use mininet_http_server::http_server;
use mininet_base::resp::HttpResponseWriter;
use mininet_http_server::HttpContext;
use mininet_base::std::StdTcpSocket;
use time::OffsetDateTime;

fn now_to_ntp() -> NtpEpochTime {
    let now = OffsetDateTime::now_utc();
    let t = now.unix_timestamp();
    NtpEpochTime::from_unix_seconds(t as u64)
}

fn main() -> Result<(), TcpError> {

    let example = async {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!());

        let mut stack = StdTcpStack;

        {
            let sntp_server_addr = stack.get_socket_address(&"0.pool.ntp.org:123").await?;
            let offset = mininet_sntp_client::client::get_sntp_time_offset(&mut stack, sntp_server_addr, now_to_ntp).await?;
            info!(logger, "Offset: {:?}", offset);
        }

        Ok(())
    };

    async_std::task::block_on(example)
}