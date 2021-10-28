
#[derive(Debug)]
pub struct HttpServerRequest {
    pub method: Option<String>,
    pub path: Option<String>,
    pub body: Vec<u8>,
    pub headers: Vec<HttpServerHeader>
}

#[derive(Debug)]
pub struct HttpServerHeader {
    pub name: String,
    pub value: String
}
