#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Clone, Default)]
#[serde(untagged)]
pub enum StringOrF64 {
    #[default]
    None,
    Str(String),
    Num(f64),
}

impl StringOrF64 {
    pub fn as_f64(&self) -> f64 {
        match self {
            StringOrF64::None => 0.0,
            StringOrF64::Str(s) => s.parse().unwrap_or(0.0),
            StringOrF64::Num(n) => *n,
        }
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub bestblockhash: String,
    pub difficulty: f64,
    #[serde(default)]
    pub time: u64,
    #[serde(default)]
    pub mediantime: u64,
    pub verificationprogress: f64,
    pub initialblockdownload: bool,
    pub size_on_disk: u64,
    pub pruned: bool,
    #[serde(default)]
    pub warnings: Warnings,
}

#[derive(Deserialize, Clone, Default)]
pub struct NetworkInfo {
    pub version: u64,
    pub subversion: String,
    pub protocolversion: u64,
    pub connections: u64,
    #[serde(default)]
    pub connections_in: u64,
    #[serde(default)]
    pub connections_out: u64,
    #[serde(default)]
    pub networkactive: bool,
    #[serde(default)]
    pub relayfee: f64,
    #[serde(default)]
    pub networks: Vec<NetworkEntry>,
    #[serde(default)]
    pub localservicesnames: Vec<String>,
    #[serde(default)]
    pub localaddresses: Vec<LocalAddress>,
    #[serde(default)]
    pub warnings: Warnings,
}

#[derive(Deserialize, Clone, Default)]
pub struct NetworkEntry {
    pub name: String,
    pub limited: bool,
    pub reachable: bool,
    #[serde(default)]
    pub proxy: String,
}

#[derive(Deserialize, Clone, Default)]
pub struct LocalAddress {
    pub address: String,
    pub port: u16,
    #[serde(default)]
    pub score: u64,
}

#[derive(Deserialize, Clone, Default)]
pub struct MempoolInfo {
    #[serde(default)]
    pub loaded: bool,
    pub size: u64,
    pub bytes: u64,
    pub usage: u64,
    #[serde(default)]
    pub total_fee: StringOrF64,
    pub maxmempool: u64,
    #[serde(default)]
    pub mempoolminfee: StringOrF64,
    #[serde(default)]
    pub minrelaytxfee: StringOrF64,
    #[serde(default)]
    pub unbroadcastcount: u64,
}

#[derive(Deserialize, Clone, Default)]
pub struct MiningInfo {
    pub blocks: u64,
    pub difficulty: f64,
    pub networkhashps: f64,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub warnings: Warnings,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct PeerInfo {
    pub id: i64,
    pub addr: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub subver: String,
    #[serde(default)]
    pub version: u64,
    pub inbound: bool,
    #[serde(default)]
    pub bytessent: u64,
    #[serde(default)]
    pub bytesrecv: u64,
    #[serde(default)]
    pub synced_headers: i64,
    #[serde(default)]
    pub synced_blocks: i64,
    pub pingtime: Option<f64>,
    #[serde(default)]
    pub conntime: u64,
    #[serde(default)]
    pub connection_type: String,
    #[serde(default)]
    pub transport_protocol_type: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Deserialize, Clone, Default)]
pub struct BlockStats {
    pub height: u64,
    pub txs: u64,
    pub total_size: u64,
    pub total_weight: u64,
    pub avgfeerate: u64,
    pub time: u64,
}

#[derive(Deserialize, Clone, Default)]
pub struct MempoolEntry {
    pub vsize: u64,
    pub weight: u64,
    pub time: u64,
    #[serde(default)]
    pub height: u64,
    #[serde(default)]
    pub descendantcount: u64,
    #[serde(default)]
    pub ancestorcount: u64,
    #[serde(default)]
    pub fees: MempoolFees,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub spentby: Vec<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct MempoolFees {
    #[serde(default)]
    pub base: StringOrF64,
    #[serde(default)]
    pub modified: StringOrF64,
    #[serde(default)]
    pub ancestor: StringOrF64,
    #[serde(default)]
    pub descendant: StringOrF64,
}

#[derive(Deserialize, Clone, Default)]
pub struct RawTransaction {
    pub txid: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub vsize: u64,
    #[serde(default)]
    pub weight: u64,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub locktime: u64,
    #[serde(default)]
    pub vin: Vec<TxInput>,
    #[serde(default)]
    pub vout: Vec<TxOutput>,
    pub blockhash: Option<String>,
    pub confirmations: Option<u64>,
    pub blocktime: Option<u64>,
    pub time: Option<u64>,
}

#[derive(Deserialize, Clone, Default)]
pub struct TxInput {
    pub txid: Option<String>,
    pub vout: Option<u64>,
    pub coinbase: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct TxOutput {
    #[serde(default)]
    pub value: StringOrF64,
    pub n: u64,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Warnings {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for Warnings {
    fn default() -> Self {
        Warnings::Multiple(Vec::new())
    }
}

impl Warnings {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Warnings::Single(s) if s.is_empty() => Vec::new(),
            Warnings::Single(s) => vec![s.clone()],
            Warnings::Multiple(v) => v.clone(),
        }
    }
}
