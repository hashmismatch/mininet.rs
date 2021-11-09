use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct OpenApi {
    #[serde(rename="openapi")]
    pub openapi_version: Cow<'static, str>,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: BTreeMap<String, Path>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Info {
    pub title: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub version: Cow<'static, str>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub url: Cow<'static, str>,
    pub description: Cow<'static, str>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Path {
    #[serde(flatten)]
    pub methods: BTreeMap<Cow<'static, str>, PathMethod>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PathMethod {
    pub summary: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,

    #[serde(rename="requestBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: BTreeMap<Cow<'static, str>, Response>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Response {
    pub description: Cow<'static, str>,    
    pub content: BTreeMap<Cow<'static, str>, ResponseContent>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResponseContent {
    pub schema: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RequestBody {
    pub required: bool,
    pub content: BTreeMap<Cow<'static, str>, RequestContent>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RequestContent {
    pub schema: serde_json::Value,
}