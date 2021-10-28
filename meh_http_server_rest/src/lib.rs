use meh_http_common::stack::TcpSocket;
use meh_http_common::resp::{HttpResponseWriter, HttpStatusCodes};
use meh_http_server::HttpContext;



pub async fn rest_handler<S>(ctx: HttpContext<S>)
    where S: TcpSocket + Send
{

    match ctx.request.path.as_deref() {
        Some("/") | None => {
            ctx.http_ok("text/html", "<h1>Root?</h1>").await;
        },
        _ => {
            ctx.http_reply(HttpStatusCodes::NotFound.into(), "text/html", "<h1>Not Found!</h1>").await;
        }
    }    
}

/*
H: Fn(HttpContext<L::TcpSocket>) -> Fut,
    Fut: Future<Output = ()>,
*/