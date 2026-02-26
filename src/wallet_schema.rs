use serde::Deserialize;

#[derive(Deserialize)]
struct OpenRpc {
    methods: Vec<RawMethod>,
}

#[derive(Deserialize)]
struct RawMethod {
    name: String,
    description: Option<String>,
    params: Vec<RawParam>,
    #[serde(rename = "x-bitcoin-category")]
    category: Option<String>,
}

#[derive(Deserialize)]
struct RawParam {
    name: String,
    description: Option<String>,
    required: Option<bool>,
    schema: Option<RawSchema>,
}

#[derive(Deserialize)]
struct RawSchema {
    #[serde(rename = "type")]
    schema_type: Option<String>,
}

pub struct RpcMethod {
    pub name: String,
    pub description: String,
    pub params: Vec<RpcParam>,
}

pub struct RpcParam {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub schema_type: String,
}

pub fn load_wallet_methods() -> Vec<RpcMethod> {
    load_methods(|cat| cat == Some("wallet"))
}

pub fn load_non_wallet_methods() -> Vec<RpcMethod> {
    load_methods(|cat| cat != Some("wallet"))
}

fn load_methods(filter: impl Fn(Option<&str>) -> bool) -> Vec<RpcMethod> {
    let json = include_str!("../openrpc.json");
    let spec: OpenRpc = serde_json::from_str(json).expect("invalid openrpc.json");

    let mut methods: Vec<RpcMethod> = spec
        .methods
        .into_iter()
        .filter(|m| filter(m.category.as_deref()))
        .map(|m| RpcMethod {
            name: m.name,
            description: m.description.unwrap_or_default(),
            params: m
                .params
                .into_iter()
                .map(|p| RpcParam {
                    name: p.name,
                    description: p.description.unwrap_or_default(),
                    required: p.required.unwrap_or(false),
                    schema_type: p
                        .schema
                        .and_then(|s| s.schema_type)
                        .unwrap_or_else(|| "any".into()),
                })
                .collect(),
        })
        .collect();

    methods.sort_by(|a, b| a.name.cmp(&b.name));
    methods
}
