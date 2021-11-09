
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

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Head,
    Options,
    Put,
    Post,
    Delete
}

impl HttpMethod {
    pub fn to_http(&self) -> &'static str {
        match *self {
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Put => "PUT",
            HttpMethod::Post => "POST",
            HttpMethod::Delete => "DELETE"
        }
    }

    pub fn from_http(&self, s: &str) -> Option<Self> {
        let method = match s {
            "GET" => HttpMethod::Get,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            "PUT" => HttpMethod::Put,
            "POST" => HttpMethod::Post,
            "DELETE" => HttpMethod::Delete,
            _ => return None
        };
        Some(method)
    }
}