use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

use meh_http_common::{req::HttpServerHeader, resp::HttpStatusCodes};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use slog::{debug, info, o, trace, warn};

use crate::middleware::{HttpMiddleware, HttpMiddlewareRunner};
use crate::{
    middleware::{HttpMiddlewareContext, Null},
    middleware_chain::Chain,
    middleware_fn::HttpMidlewareFnFut,
    openapi::{
        Info, OpenApi, Path, PathMethod, RequestBody, RequestContent, Response, ResponseContent,
        Server,
    },
    response_builder::HttpResponseBuilder,
    HandlerResult, HandlerResultOk, RestErrorContext, RestResult,
};
use async_trait::async_trait;

struct OpenApiContext {
    is_openapi_request: bool,
    apis: HashMap<Cow<'static, str>, OpenApiContextApi>,
}

struct OpenApiContextApi {
    path: Cow<'static, str>,
    paths: HashMap<Cow<'static, str>, Path>,
    combined_getters: Vec<OpenApiGetter>,
}

struct OpenApiGetter {
    id: Cow<'static, str>,
    getter: Box<dyn FnOnce() -> RestResult<serde_json::Value> + Send + Sync>,
    json_schema_type_def: serde_json::Value,
}

pub struct QuickRestOpenApiMiddleware<C> {
    pub _context: PhantomData<C>,
    pub info: Info,
    pub servers: Vec<Server>,
}

impl<C> QuickRestOpenApiMiddleware<C>
where
    C: HttpMiddlewareContext,
{
    async fn whole_api(self, mut ctx: HttpResponseBuilder<C>) -> HandlerResult<C> {
        let req_path = ctx.request.path.clone();

        let openapi = ctx.extras.get_mut::<OpenApiContext>();
        let openapi = if let Some(openapi) = openapi {
            openapi
        } else {
            return Ok(ctx.into());
        };

        let mut paths = HashMap::new();
        for (api_id, api) in &openapi.apis {
            let api_url = format!("{}", api_id);

            // create the all endpoint
            let all_properties = api
                .combined_getters
                .iter()
                .map(|g| {
                    let id = g.id.to_string();
                    (id, g.json_schema_type_def.clone())
                })
                .collect::<serde_json::Map<String, Value>>();

            let all_schema = json!(
                {
                    "type": "object",
                    "properties":
                        all_properties
                }
            );

            paths.insert(
                api_url.into(),
                Path {
                    methods: [(
                        "get".into(),
                        PathMethod {
                            summary: "All values".into(),
                            description: Some("All of the values in one object".into()),
                            request_body: None,
                            responses: [(
                                "200".into(),
                                Response {
                                    description: "The contents".into(),
                                    content: [(
                                        "application/json".into(),
                                        ResponseContent { schema: all_schema },
                                    )]
                                    .into_iter()
                                    .collect(),
                                },
                            )]
                            .into_iter()
                            .collect(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            );

            paths.extend(api.paths.clone());
        }

        // handle the GET request
        for (api_id, api) in &mut openapi.apis {
            let api_url = format!("{}", api_id);

            if req_path.as_deref() == Some(&api_url) {
                let mut map = Map::new();
                for g in api.combined_getters.drain(..) {
                    map.insert(g.id.clone().into_owned(), (g.getter)().unwrap());
                }
                let json = serde_json::to_string_pretty(&Value::Object(map)).unwrap();
                let r = ctx
                    .response(HttpStatusCodes::Ok, "application/json".into(), Some(&json))
                    .await;
                match r {
                    Ok(ok) => {
                        return Ok(ok.into());
                    }
                    Err(e) => {
                        return Err(RestErrorContext {
                            error: e,
                            ctx: None,
                        });
                    }
                }
            }
        }

        // handle the big openapi request
        if req_path.as_deref() == Some("/?openapi") {
            let o = OpenApi {
                openapi_version: "3.0.0".into(),
                info: self.info,
                servers: self.servers,
                paths,
            };
            let json = serde_json::to_string_pretty(&o).unwrap();
            let r = ctx
                .response(HttpStatusCodes::Ok, "application/json".into(), Some(&json))
                .await;
            match r {
                Ok(ok) => {
                    return Ok(ok.into());
                }
                Err(e) => {
                    return Err(RestErrorContext {
                        error: e,
                        ctx: None,
                    });
                }
            }
        }

        Ok(ctx.into())
    }
}

#[async_trait]
impl<C> HttpMiddleware for QuickRestOpenApiMiddleware<C>
where
    C: HttpMiddlewareContext,
{
    type Context = C;

    async fn handle<N>(
        self,
        mut ctx: HttpResponseBuilder<Self::Context>,
        next: N,
    ) -> HandlerResult<Self::Context>
    where
        N: HttpMiddlewareRunner<Context = Self::Context>,
    {
        let is_openapi_request = ctx.request.path.as_ref().map(|p| p.ends_with("?openapi"))
            == Some(true)
            && ctx.request.method.as_ref().map(|s| s.as_str()) == Some("GET");

        let open_api = OpenApiContext {
            is_openapi_request,
            apis: HashMap::new(),
        };

        ctx.extras.insert(open_api);

        match next.run(ctx).await {
            Ok(HandlerResultOk::Complete(c)) => Ok(c.into()),
            Ok(HandlerResultOk::Pass(ctx)) => self.whole_api(ctx).await,
            Err(e) => Err(e),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ValueDto<T>
where
    T: Serialize,
{
    pub value: T,
}

pub struct QuickRestValue<T>
where
    T: Serialize + Send + DeserializeOwned + core::fmt::Debug,
{
    pub api: Cow<'static, str>,
    pub id: Cow<'static, str>,
    pub get: Option<Box<dyn FnOnce() -> RestResult<T> + Send + Sync>>,
    pub set: Option<Box<dyn FnOnce(T) -> RestResult + Send>>,
}

impl<T> QuickRestValue<T>
where
    T: Serialize + Send + DeserializeOwned + core::fmt::Debug,
{
    pub fn new_getter<G>(api: Cow<'static, str>, id: Cow<'static, str>, getter: G) -> Self
    where
        G: FnOnce() -> RestResult<T> + Send + Sync + 'static,
    {
        QuickRestValue {
            api,
            id,
            get: Some(Box::new(getter)),
            set: None,
        }
    }

    pub fn new_setter<S>(api: Cow<'static, str>, id: Cow<'static, str>, setter: S) -> Self
    where
        S: FnOnce(T) -> RestResult + Send + 'static,
    {
        QuickRestValue {
            api,
            id,
            get: None,
            set: Some(Box::new(setter)),
        }
    }

    pub fn new_getter_and_setter<G, S>(
        api: Cow<'static, str>,
        id: Cow<'static, str>,
        getter: G,
        setter: S,
    ) -> Self
    where
        G: FnOnce() -> RestResult<T> + Send + Sync + 'static,
        S: FnOnce(T) -> RestResult + Send + 'static,
    {
        QuickRestValue {
            api,
            id,
            get: Some(Box::new(getter)),
            set: Some(Box::new(setter)),
        }
    }
}

async fn quick_rest_value_fn<C, T>(
    mut ctx: HttpResponseBuilder<C>,
    v: QuickRestValue<T>,
) -> HandlerResult<C>
where
    C: HttpMiddlewareContext,
    T: OpenApiType,
{
    let p = format!("{}/{}", v.api, v.id);
    if ctx.request.path == Some(p) {
        let l = ctx.logger.new(o!("id" => v.id.to_string()));
        debug!(l, "Matched with Quick REST.");
        let method = ctx.request.method.as_ref().cloned();

        if let Some("OPTIONS") = method.as_deref() {
            ctx.additional_headers.push(HttpServerHeader {
                name: "Allow".into(),
                value: "OPTIONS, GET, POST".into(),
            });
            ctx.additional_headers.push(HttpServerHeader {
                name: "Access-Control-Allow-Methods".into(),
                value: "OPTIONS, GET, POST".into(),
            });
            ctx.additional_headers.push(HttpServerHeader {
                name: "Access-Control-Allow-Headers".into(),
                value: "*".into(),
            });
            let r = ctx.response(HttpStatusCodes::NoContent, None, None).await?;
            return Ok(r.into());
        }

        if let Some(getter) = v.get {
            match method.as_deref() {
                Some("GET") => {
                    let value = (getter)()?;
                    let dto = ValueDto { value };

                    let json = match serde_json::to_string_pretty(&dto) {
                        Ok(j) => j,
                        Err(e) => {
                            return Err(RestErrorContext {
                                error: e.into(),
                                ctx: Some(ctx),
                            });
                        }
                    };

                    debug!(
                        l,
                        "Replying with the JSON value. Current value, as debug format: {:?}",
                        dto.value
                    );
                    let r = ctx
                        .response(HttpStatusCodes::Ok, "application/json".into(), Some(&json))
                        .await?;
                    return Ok(r.into());
                }
                _ => (),
            }
        }

        if let Some(setter) = v.set {
            match method.as_deref() {
                Some("POST" | "PUT") => {
                    let dto = serde_json::from_slice::<ValueDto<T>>(&ctx.request.body);
                    match dto {
                        Ok(dto) => {
                            debug!(
                                l,
                                "Set the new value. New value, as debug format: {:?}", dto.value
                            );
                            (setter)(dto.value)?;
                            let r = ctx.response(HttpStatusCodes::NoContent, None, None).await?;
                            return Ok(r.into());
                        }
                        Err(e) => {
                            warn!(ctx.logger, "Failed to deserialize the body: {:?}", e);
                            if let Ok(body) = core::str::from_utf8(&ctx.request.body) {
                                debug!(ctx.logger, "Body as a string: {body}", body = body);
                            }

                            return Err(RestErrorContext {
                                error: e.into(),
                                ctx: Some(ctx),
                            });
                        }
                    }
                }
                _ => (),
            }
        }
    } else {
        if let Some(getter) = v.get {
            let g = OpenApiGetter {
                id: v.id,
                getter: Box::new(move || {
                    let val = (getter)()?;
                    Ok(serde_json::to_value(val)?)
                }),
                json_schema_type_def: T::json_schema_definition(),
            };

            if let Some(openapi_ctx) = ctx.extras.get_mut::<OpenApiContext>() {
                openapi_ctx
                    .apis
                    .entry(v.api.clone())
                    .or_insert_with(|| OpenApiContextApi {
                        path: v.api.clone(),
                        paths: HashMap::new(),
                        combined_getters: vec![],
                    });
                openapi_ctx.apis.entry(v.api.clone()).and_modify(|e| {
                    e.combined_getters.push(g);
                });
            }
        }
    }

    Ok(ctx.into())
}

pub fn quick_rest_value<C, T>(q: QuickRestValue<T>) -> HttpMidlewareFnFut<C>
where
    C: HttpMiddlewareContext + 'static,
    T: OpenApiType,
{
    HttpMidlewareFnFut::new(|ctx| quick_rest_value_fn(ctx, q))
}

pub trait OpenApiType: Serialize + DeserializeOwned + Send + core::fmt::Debug + 'static {
    fn json_schema_definition() -> serde_json::Value;
}

impl OpenApiType for usize {
    fn json_schema_definition() -> serde_json::Value {
        let min = usize::MIN;
        let max = usize::MAX;
        json!({
            "type": "integer",
            "minimum": min,
            "maximum": max
        })
    }
}

impl OpenApiType for String {
    fn json_schema_definition() -> serde_json::Value {
        json!({
            "type": "string"
        })
    }
}

async fn quick_rest_value_openapi_fn<C, T>(
    mut ctx: HttpResponseBuilder<C>,
    api: Cow<'static, str>,
    id: Cow<'static, str>,
) -> HandlerResult<C>
where
    C: HttpMiddlewareContext,
    T: OpenApiType,
{
    let openapi = ctx.extras.get_mut::<OpenApiContext>();
    let openapi = if let Some(openapi) = openapi {
        openapi
    } else {
        return Ok(ctx.into());
    };

    if openapi.is_openapi_request {
        {
            let ty_def = T::json_schema_definition();

            let schema = serde_json::json!(
                {
                    "type": "object",
                    "properties": {
                        "value": ty_def
                    }
                }
            );
            let mut methods = HashMap::new();

            // get
            {
                let mut response_contents = HashMap::new();
                response_contents.insert(
                    "application/json".into(),
                    ResponseContent {
                        schema: schema.clone(),
                    },
                );

                let mut responses = HashMap::new();
                responses.insert(
                    "200".into(),
                    Response {
                        description: "Response for the current value".into(),
                        content: response_contents,
                    },
                );

                methods.insert(
                    "get".into(),
                    PathMethod {
                        description: None,
                        summary: "Get the current value".into(),
                        responses,
                        request_body: None,
                    },
                );
            }

            // set
            {
                let mut responses = HashMap::new();
                responses.insert(
                    "204".into(),
                    Response {
                        description: "Successfully set the new value".into(),
                        content: HashMap::new(),
                    },
                );

                methods.insert(
                    "post".into(),
                    PathMethod {
                        description: None,
                        summary: "Try to set a new value for this variable".into(),
                        responses,
                        request_body: Some(RequestBody {
                            required: true,
                            content: [(
                                "application/json".into(),
                                RequestContent {
                                    schema: schema.clone(),
                                },
                            )]
                            .into_iter()
                            .collect(),
                        }),
                    },
                );
            }

            let p: Cow<str> = format!("{}/{}", api, id).into();
            openapi.apis.entry(api.clone()).or_insert_with({
                let api = api.clone();
                move || OpenApiContextApi {
                    path: api.clone().into(),
                    paths: HashMap::new(),
                    combined_getters: vec![],
                }
            });
            openapi.apis.entry(api.clone()).and_modify(|e| {
                e.paths.insert(p, Path { methods });
            });
        };
    }

    Ok(ctx.into())
}

pub fn quick_rest_value_with_openapi<C, T>(
    q: QuickRestValue<T>,
) -> Chain<C, HttpMidlewareFnFut<C>, Chain<C, HttpMidlewareFnFut<C>, Null<C>>>
where
    C: HttpMiddlewareContext + 'static,
    T: OpenApiType,
{
    let api = q.api.clone();
    let id = q.id.clone();

    let val = HttpMidlewareFnFut::new(|ctx| quick_rest_value_fn(ctx, q));

    let openapi =
        HttpMidlewareFnFut::new(move |ctx| quick_rest_value_openapi_fn::<C, T>(ctx, api, id));

    Chain::new(val).chain(openapi)
}
