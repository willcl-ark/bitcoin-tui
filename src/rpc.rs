use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::rpc_types::*;

pub struct RpcClient {
    url: String,
    auth: Auth,
    client: Client,
}

enum Auth {
    UserPass { user: String, pass: String },
    Cookie(PathBuf),
}

impl RpcClient {
    pub fn new(
        host: &str,
        port: u16,
        cookie: Option<PathBuf>,
        user: Option<&str>,
        pass: Option<&str>,
    ) -> Self {
        let url = format!("http://{}:{}", host, port);
        let auth = if let Some(user) = user {
            Auth::UserPass {
                user: user.to_string(),
                pass: pass.unwrap_or("").to_string(),
            }
        } else {
            Auth::Cookie(cookie.unwrap_or_else(|| default_cookie_path(None)))
        };
        RpcClient {
            url,
            auth,
            client: Client::new(),
        }
    }

    async fn auth_header(&self) -> Result<String, String> {
        match &self.auth {
            Auth::UserPass { user, pass } => Ok(format!(
                "Basic {}",
                BASE64.encode(format!("{}:{}", user, pass))
            )),
            Auth::Cookie(path) => {
                let contents = tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| format!("Failed to read cookie file {}: {}", path.display(), e))?;
                Ok(format!("Basic {}", BASE64.encode(contents.trim())))
            }
        }
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T, String> {
        let auth = self.auth_header().await?;
        let body = json!({
            "jsonrpc": "1.0",
            "id": method,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("RPC connection failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(format!("RPC error ({}): {}", status, text));
        }

        let parsed: Value =
            serde_json::from_str(&text).map_err(|e| format!("Invalid JSON: {}", e))?;

        if let Some(err) = parsed.get("error")
            && !err.is_null()
        {
            return Err(format!("RPC error: {}", err));
        }

        serde_json::from_value(parsed["result"].clone())
            .map_err(|e| format!("Failed to parse {}: {}", method, e))
    }

    pub async fn call_raw(
        &self,
        method: &str,
        params: Value,
        wallet: Option<&str>,
    ) -> Result<Value, String> {
        let auth = self.auth_header().await?;
        let url = match wallet {
            Some(name) if !name.is_empty() => format!("{}/wallet/{}", self.url, name),
            _ => self.url.clone(),
        };
        let body = json!({
            "jsonrpc": "1.0",
            "id": method,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("RPC connection failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if !status.is_success() {
            return Err(format!("RPC error ({}): {}", status, text));
        }

        let parsed: Value =
            serde_json::from_str(&text).map_err(|e| format!("Invalid JSON: {}", e))?;

        if let Some(err) = parsed.get("error")
            && !err.is_null()
        {
            return Err(format!("RPC error: {}", err));
        }

        Ok(parsed["result"].clone())
    }

    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo, String> {
        self.call("getblockchaininfo", json!([])).await
    }

    pub async fn get_network_info(&self) -> Result<NetworkInfo, String> {
        self.call("getnetworkinfo", json!([])).await
    }

    pub async fn get_mempool_info(&self) -> Result<MempoolInfo, String> {
        self.call("getmempoolinfo", json!([])).await
    }

    pub async fn get_mining_info(&self) -> Result<MiningInfo, String> {
        self.call("getmininginfo", json!([])).await
    }

    pub async fn get_peer_info(&self) -> Result<Vec<PeerInfo>, String> {
        self.call("getpeerinfo", json!([])).await
    }

    pub async fn get_block_stats(&self, height: u64) -> Result<BlockStats, String> {
        self.call(
            "getblockstats",
            json!([
                height,
                [
                    "height",
                    "txs",
                    "total_size",
                    "total_weight",
                    "avgfeerate",
                    "time"
                ]
            ]),
        )
        .await
    }

    pub async fn get_mempool_entry(&self, txid: &str) -> Result<MempoolEntry, String> {
        self.call("getmempoolentry", json!([txid])).await
    }

    pub async fn get_raw_transaction(&self, txid: &str) -> Result<RawTransaction, String> {
        self.call("getrawtransaction", json!([txid, 1])).await
    }
}

pub fn default_cookie_path(network_subdir: Option<&str>) -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".bitcoin");
    if let Some(subdir) = network_subdir {
        path.push(subdir);
    }
    path.push(".cookie");
    path
}
