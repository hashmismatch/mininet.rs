use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OpenApi {
    #[serde(rename="openapi")]
    pub openapi_version: Cow<'static, str>,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: HashMap<Cow<'static, str>, Path>
}

#[derive(Serialize, Deserialize)]
pub struct Info {
    pub title: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub version: Cow<'static, str>
}

#[derive(Serialize, Deserialize)]
pub struct Server {
    pub url: Cow<'static, str>,
    pub description: Cow<'static, str>
}

#[derive(Serialize, Deserialize)]
pub struct Path {
    #[serde(flatten)]
    pub methods: HashMap<Cow<'static, str>, PathMethod>
}

#[derive(Serialize, Deserialize)]
pub struct PathMethod {
    pub summary: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,

    #[serde(rename="requestBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<Cow<'static, str>, Response>
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub description: Cow<'static, str>,    
    pub content: HashMap<Cow<'static, str>, ResponseContent>
}

#[derive(Serialize, Deserialize)]
pub struct ResponseContent {
    pub schema: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub content: HashMap<Cow<'static, str>, RequestContent>
}

#[derive(Serialize, Deserialize)]
pub struct RequestContent {
    pub schema: serde_json::Value,
}