use async_trait::async_trait;

use crate::stack::TcpError;

/*
pub trait HttpResponseWriter {
    type WriteOutput: core::future::Future<Output = ()>;

    fn write(&mut self, data: &[u8]) -> Self::WriteOutput;
}
*/

#[async_trait]
pub trait HttpResponseWriter where Self: Sized {
    async fn write(&mut self, data: &[u8]) -> Result<(), TcpError>;

    async fn http_ok(mut self, content_type: &str, body: &str) -> Result<(), TcpError> {
        self.write(b"HTTP/1.1 200 OK\r\n").await?;
        self.write(b"Content-Type: ").await?;
        self.write(content_type.as_bytes()).await?;
        self.write(b"\r\n\r\n").await?;
        self.write(body.as_bytes()).await?;

        Ok(())
    }
}