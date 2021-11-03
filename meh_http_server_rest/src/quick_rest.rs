use std::{any::TypeId, borrow::Cow, collections::HashMap};

use meh_http_common::{req::HttpServerHeader, resp::HttpStatusCodes, stack::TcpSocket};
use serde::{Deserialize, Serialize, Serializer, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use slog::{debug, info, o, trace, warn};

use crate::{HandlerResult, HttpMidlewareChain, HttpMidlewareFn, HttpMidlewareFnFut, HttpResponseBuilder, RestError, openapi::{Info, OpenApi, Path, PathMethod, RequestBody, RequestContent, Response, ResponseContent, Server}};


struct OpenApiContext {
    enabled: bool,
    is_openapi_request: bool,
    paths: HashMap<Cow<'static, str>, Path>,
    combined_getters: Vec<OpenApiGetter>
}

struct OpenApiGetter {
    id: Cow<'static, str>,
    getter: Box<dyn FnOnce() -> serde_json::Value + Send + Sync>,
    json_schema_type: Cow<'static, str>
}

impl Default for OpenApiContext {
    fn default() -> Self {
        Self { enabled: false, is_openapi_request: false, paths: HashMap::new(), combined_getters: vec![] }
    }
}

pub fn enable_open_api<S>() -> HttpMidlewareFn<S>
    where S: TcpSocket
{
    HttpMidlewareFn::new(|mut ctx: HttpResponseBuilder<S>| {        
        let is_openapi_request = 
            ctx.request.path.as_ref().map(|p| p.ends_with("?openapi")) == Some(true) && 
            ctx.request.method.as_ref().map(|s| s.as_str()) == Some("GET");

        let v = ctx.extras.get_mut::<OpenApiContext>();
        v.enabled = true;
        v.is_openapi_request = is_openapi_request;

        Ok(ctx.into())
    })
}


#[derive(Serialize, Deserialize)]
pub struct ValueDto<T>
    where T: Serialize
{
    pub value: T
}

pub struct QuickRestValue<T>
    where T: Serialize + Send + DeserializeOwned + core::fmt::Debug
{
    pub api: Cow<'static, str>,
    pub id: Cow<'static, str>,
    pub get: Option<Box<dyn FnOnce() -> T + Send + Sync>>,
    pub set: Option<Box<dyn FnOnce(T) -> () + Send>>
}

impl<T> QuickRestValue<T>
    where T: Serialize + Send + DeserializeOwned + core::fmt::Debug
{
    pub fn new_getter<G>(api: Cow<'static, str>, id: Cow<'static, str>, getter: G) -> Self
        where G: FnOnce() -> T + Send + Sync + 'static
    {
        QuickRestValue {
            api,
            id,
            get: Some(Box::new(getter)),
            set: None
        }
    }

    pub fn new_setter<S>(api: Cow<'static, str>, id: Cow<'static, str>, setter: S) -> Self
        where S: FnOnce(T) -> () + Send + 'static
    {
        QuickRestValue {
            api,
            id,
            get: None,
            set: Some(Box::new(setter))
        }
    }

    pub fn new_getter_and_setter<G, S>(api: Cow<'static, str>, id: Cow<'static, str>, getter: G, setter: S) -> Self
        where G: FnOnce() -> T + Send + Sync + 'static,
              S: FnOnce(T) -> () + Send + 'static
    {
        QuickRestValue {
            api,
            id,
            get: Some(Box::new(getter)),
            set: Some(Box::new(setter))
        }
    }
}


async fn quick_rest_value_fn<S, T>(mut ctx: HttpResponseBuilder<S>, v: QuickRestValue<T>) -> HandlerResult<S>
    where S: TcpSocket,
          T: OpenApiType
{
    let p = format!("/{}", v.id);
    if ctx.request.path == Some(p) {
        let l = ctx.logger.new(o!("id" => v.id.to_string()));
        debug!(l, "Matched with Quick REST.");
        let method = ctx.request.method.as_ref().cloned();

        if let Some("OPTIONS") = method.as_deref() {
            ctx.additional_headers.push(HttpServerHeader {
                name: "Allow".into(),
                value: "OPTIONS, GET, POST".into(),
            });
            ctx.additional_headers.push(HttpServerHeader { name: "Access-Control-Allow-Methods".into(), value: "OPTIONS, GET, POST".into() });
            ctx.additional_headers.push(HttpServerHeader { name: "Access-Control-Allow-Headers".into(), value: "*".into() });
            let r = ctx.response(HttpStatusCodes::NoContent, None, None).await?;
            return Ok(r.into());
        }

        if let Some(getter) = v.get {
            match method.as_deref() {
                Some("GET") => {
                    let value = (getter)();
                    let dto = ValueDto {
                        value
                    };

                    let json = serde_json::to_string_pretty(&dto);
                    if let Ok(json) = json {
                        debug!(l, "Replying with the JSON value. Current value, as debug format: {:?}", dto.value);
                        let r = ctx.response(HttpStatusCodes::Ok, "application/json".into(), Some(&json)).await?;
                        return Ok(r.into());
                    } else {
                        return Err(RestError::Unknown);
                    }
                }
                _ => ()
            }
        }

        if let Some(setter) = v.set {
            match method.as_deref() {
                Some("POST" | "PUT") => {
                    let dto = serde_json::from_slice::<ValueDto<T>>(&ctx.request.body);
                    match dto {
                        Ok(dto) => {
                            debug!(l, "Set the new value. New value, as debug format: {:?}", dto.value);
                            (setter)(dto.value);
                            let r = ctx.response(HttpStatusCodes::NoContent, None, None).await?;
                            return Ok(r.into());
                        },
                        Err(e) => {
                            warn!(ctx.logger, "Failed to deserialize the body: {:?}", e);
                            if let Ok(body) = core::str::from_utf8(&ctx.request.body) {
                                debug!(ctx.logger, "Body as a string: {body}", body=body);
                            }

                            let msg = format!("{:?}", e);
                            let error = json!({
                                "error": msg
                            });
                            let body = serde_json::to_string_pretty(&error).unwrap();

                            let r = ctx.response(HttpStatusCodes::BadRequest, Some("application/json".into()), Some(&body)).await?;
                            return Ok(r.into());
                        }
                    }
                },
                _ => ()
            }
        }
    } else {        
        if let Some(getter) = v.get {
            let g = OpenApiGetter {
                id: v.id,
                getter: Box::new(|| {
                    let val = (getter)();
                    serde_json::to_value(val).unwrap()
                }),
                json_schema_type: T::json_schema_type()
            };
            ctx.extras.get_mut::<OpenApiContext>().combined_getters.push(g);
        }
    }

    Ok(ctx.into())
}


pub fn quick_rest_value<S, T>(q: QuickRestValue<T>) -> HttpMidlewareFnFut<S>
    where S: TcpSocket,
          T: OpenApiType
{
    HttpMidlewareFnFut::new(|ctx| {
        quick_rest_value_fn(ctx, q)
    })
}

pub trait OpenApiType: Serialize + DeserializeOwned + Send + core::fmt::Debug + 'static {
    fn json_schema_type() -> Cow<'static, str>;
}

impl OpenApiType for usize {
    fn json_schema_type() -> Cow<'static, str> {
        "integer".into()
    }
}

impl OpenApiType for String {
    fn json_schema_type() -> Cow<'static, str> {
        "string".into()
    }
}


async fn quick_rest_value_openapi_fn<S, T>(mut ctx: HttpResponseBuilder<S>, id: Cow<'static, str>) -> HandlerResult<S>
    where S: TcpSocket,
          T: OpenApiType
{
    let openapi = ctx.extras.get_mut::<OpenApiContext>();
    
    if openapi.enabled && openapi.is_openapi_request {
        {
            let ty = T::json_schema_type();

            let schema = serde_json::json!(
                {
                    "type": "object",
                    "properties": {
                        "value": {
                            "type": ty
                        }
                    }
                }
            );
            
            let mut methods = HashMap::new();

            // get
            {
                let mut response_contents = HashMap::new();
                response_contents.insert("application/json".into(), ResponseContent {
                    schema: schema.clone()
                });

                let mut responses = HashMap::new();
                responses.insert("200".into(), Response {
                    description: "Response for the current value".into(),
                    content: response_contents
                });

                methods.insert("get".into(), PathMethod {
                    description: None,
                    summary: "Get the current value".into(),
                    responses,
                    request_body: None
                });
            }

            // set
            {
                let mut responses = HashMap::new();
                responses.insert("204".into(), Response {
                    description: "Successfully set the new value".into(),
                    content: HashMap::new()
                });

                methods.insert("post".into(), PathMethod {
                    description: None,
                    summary: "Try to set a new value for this variable".into(),
                    responses,
                    request_body: Some(RequestBody {
                        required: true,
                        content: [("application/json".into(), RequestContent {
                            schema: schema.clone()
                        })].into_iter().collect()
                    })
                });
            }

            let p: Cow<str> = format!("/{}", id).into();
            openapi.paths.insert(p, Path { methods });
        };
    }

    Ok(ctx.into())
}

pub fn quick_rest_value_with_openapi<S, T>(q: QuickRestValue<T>) -> HttpMidlewareChain<HttpMidlewareFnFut<S>, HttpMidlewareFnFut<S>, S>
    where S: TcpSocket,
          T: OpenApiType
{
    let id = q.id.clone();

    let val = HttpMidlewareFnFut::new(|ctx| {
        quick_rest_value_fn(ctx, q)
    });

    let openapi = HttpMidlewareFnFut::new(move |ctx| {
        quick_rest_value_openapi_fn::<S, T>(ctx, id)
    });

    HttpMidlewareChain::new_pair(val, openapi)
}

async fn openapi_handler_fn<S>(mut ctx: HttpResponseBuilder<S>) -> HandlerResult<S>
    where S: TcpSocket
{
    let openapi = ctx.extras.get_mut::<OpenApiContext>();
    if openapi.enabled && openapi.is_openapi_request {
        let mut paths = HashMap::new();
        std::mem::swap(&mut paths, &mut openapi.paths);

        // create the all endpoint
        {
            let all_properties = openapi.combined_getters
                .iter()
                .map(|g| {
                    let id = g.id.to_string();
                    let ty = g.json_schema_type.to_string();

                    (id, serde_json::json!({ "type": ty }))
                }).collect::<serde_json::Map<String, Value>>();

                let all_schema = json!(
                {
                    "type": "object",
                    "properties": 
                        all_properties
                    
                }
            );

            paths.insert("/all".into(), Path {
                methods: [
                    ("get".into(),
                    PathMethod {
                        summary: "All values".into(),
                        description: Some("All of the values in one object".into()),
                        request_body: None,
                        responses: [
                            ("200".into(), Response {
                                description: "The contents".into(),
                                content: [
                                    ("application/json".into(),
                                    ResponseContent {
                                        schema: all_schema
                                    })].into_iter().collect()
                            })].into_iter().collect()
                    }
                )
                ].into_iter().collect()
            });
        }

        let o = OpenApi {
            openapi_version: "3.0.0".into(),
            info: Info {
                title: "Quick".into(),
                description: "API".into(),
                version: "0.1.0".into()
            },
            servers: vec![
                Server {
                    description: "here".into(),
                    url: "http://localhost:8080".into()
                }
            ],
            paths
        };

        let json = serde_json::to_string_pretty(&o);
        if let Ok(json) = json {
            let r = ctx.response(HttpStatusCodes::Ok, "application/json".into(), Some(&json)).await?;;
            return Ok(r.into());
        }
    }


    // joint request?
    if ctx.request.path.as_deref() == Some("/all") {
        let mut map = Map::new();
        let c = ctx.extras.get_mut::<OpenApiContext>();

        for g in c.combined_getters.drain(..) {
            map.insert(g.id.into_owned(), (g.getter)());
        }
        
        let json = serde_json::to_string_pretty(&Value::Object(map));
        if let Ok(json) = json {
            let r = ctx.response(HttpStatusCodes::Ok, "application/json".into(), Some(&json)).await?;
            return Ok(r.into())
        }
    }    

    Ok(ctx.into())
}

pub fn openapi_final_handler<S>() -> HttpMidlewareFnFut<S>
    where S: TcpSocket
{
    HttpMidlewareFnFut::new(|ctx| {
        openapi_handler_fn(ctx)
    })
}
