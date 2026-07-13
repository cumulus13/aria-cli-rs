// File: src/rpc.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Blocking JSON-RPC client for aria2 (https://aria2.github.io/manual/en/html/aria2c.html#rpc-interface).

use crate::error::{AriaError, Result};
use log::{debug, error as log_error};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone)]
pub struct Aria2Client {
    pub rpc_url: String,
    pub secret: Option<String>,
    http: reqwest::blocking::Client,
}

impl Aria2Client {
    pub fn new(rpc_url: impl Into<String>, secret: Option<String>) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build HTTP client");
        Self {
            rpc_url: rpc_url.into(),
            secret,
            http,
        }
    }

    fn params_with_secret(&self, mut params: Vec<Value>) -> Vec<Value> {
        if let Some(secret) = &self.secret {
            params.insert(0, json!(format!("token:{secret}")));
        }
        params
    }

    fn call(&self, method: &str, params: Vec<Value>) -> Result<Value> {
        debug!("aria2 RPC -> {method} {params:?}");
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "aria-cli",
            "method": method,
            "params": params,
        });
        let resp = self.http.post(&self.rpc_url).json(&payload).send()?;
        let resp = resp.error_for_status()?;
        let body: Value = resp.json()?;
        if let Some(err) = body.get("error") {
            let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string();
            log_error!("aria2 RPC error [{code}] {message} (method={method})");
            return Err(AriaError::Rpc { code, message });
        }
        Ok(body.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Best-effort call: returns `Ok(false)` instead of propagating "GID not found" (code 1)
    /// errors, so callers can fall through to alternate RPC methods.
    fn call_best_effort(&self, method: &str, params: Vec<Value>) -> Result<bool> {
        match self.call(method, params) {
            Ok(_) => Ok(true),
            Err(AriaError::Rpc { code, .. }) if code == 1 => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn add_uri(
        &self,
        urls: &[String],
        headers: Option<&[String]>,
        cookie: Option<&str>,
        dir: Option<&str>,
    ) -> Result<String> {
        let mut options = serde_json::Map::new();
        if let Some(h) = headers {
            if !h.is_empty() {
                options.insert("header".into(), json!(h));
            }
        }
        if let Some(c) = cookie {
            options.insert("cookie".into(), json!(c));
        }
        if let Some(d) = dir {
            options.insert("dir".into(), json!(d));
        }

        let params = self.params_with_secret(vec![json!(urls), Value::Object(options)]);
        let result = self.call("aria2.addUri", params)?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    pub fn tell_active(&self) -> Result<Vec<Value>> {
        let params = self.params_with_secret(vec![]);
        let result = self.call("aria2.tellActive", params)?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    pub fn tell_waiting(&self, offset: i64, num: i64) -> Result<Vec<Value>> {
        let params = self.params_with_secret(vec![json!(offset), json!(num)]);
        let result = self.call("aria2.tellWaiting", params)?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    pub fn tell_stopped(&self, offset: i64, num: i64) -> Result<Vec<Value>> {
        let params = self.params_with_secret(vec![json!(offset), json!(num)]);
        let result = self.call("aria2.tellStopped", params)?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    pub fn tell_status(&self, gid: &str) -> Result<Value> {
        let params = self.params_with_secret(vec![json!(gid)]);
        self.call("aria2.tellStatus", params)
    }

    /// All downloads (active + waiting + stopped), each tagged with a `category` field,
    /// mirroring the Python tool's `get_downloads_data`.
    pub fn all_downloads(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for (label, items) in [
            ("active", self.tell_active().unwrap_or_default()),
            ("waiting", self.tell_waiting(0, 1000).unwrap_or_default()),
            ("stopped", self.tell_stopped(0, 1000).unwrap_or_default()),
        ] {
            for mut item in items {
                if let Value::Object(ref mut map) = item {
                    map.insert("category".into(), json!(label));
                }
                out.push(item);
            }
        }
        out
    }

    /// Tries `remove`, then `forceRemove`, then `removeDownloadResult`, matching the
    /// Python implementation's fallback chain for stubborn GIDs.
    pub fn remove_download(&self, gid: &str) -> Result<bool> {
        for method in ["aria2.remove", "aria2.forceRemove", "aria2.removeDownloadResult"] {
            let params = self.params_with_secret(vec![json!(gid)]);
            if self.call_best_effort(method, params)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn pause_download(&self, gid: &str) -> Result<bool> {
        let params = self.params_with_secret(vec![json!(gid)]);
        self.call_best_effort("aria2.pause", params)
    }

    pub fn resume_download(&self, gid: &str) -> Result<bool> {
        let params = self.params_with_secret(vec![json!(gid)]);
        if self.call_best_effort("aria2.unpause", params)? {
            return Ok(true);
        }
        let params = self.params_with_secret(vec![]);
        self.call_best_effort("aria2.unpauseAll", params)
    }

    /// Fetches the original URI(s) for `gid`, removes it, and re-adds it as a fresh
    /// download — aria2 has no native "retry" RPC method.
    pub fn retry_download(&self, gid: &str) -> Result<bool> {
        let status = self.tell_status(gid)?;
        let files = status.get("files").and_then(Value::as_array).cloned().unwrap_or_default();
        let uris: Vec<String> = files
            .iter()
            .filter_map(|f| {
                f.get("uris")
                    .and_then(Value::as_array)
                    .and_then(|u| u.first())
                    .and_then(|u| u.get("uri"))
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .collect();
        if uris.is_empty() {
            return Ok(false);
        }
        self.remove_download(gid).ok();
        let params = self.params_with_secret(vec![json!(uris), json!({})]);
        let result = self.call("aria2.addUri", params)?;
        Ok(result.as_str().is_some())
    }

    pub fn purge_all(&self) -> usize {
        let downloads = self.all_downloads();
        let mut count = 0;
        for task in &downloads {
            if let Some(gid) = task.get("gid").and_then(Value::as_str) {
                if self.remove_download(gid).unwrap_or(false) {
                    count += 1;
                }
            }
        }
        count
    }
}
