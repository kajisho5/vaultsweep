//! MCP サーバー（標準入出力・1行に1つの JSON）
//! - 新しい版（2026-07-28）：毎回 _meta に版と相手の機能が入る。server/discover に答える。
//! - 古い版（2025-11-25 以前）：initialize から始まる。こちらにも答える。
//! パスワード・ユーザー名・メモ欄は絶対に返さない。片付け・変更は「依頼」を置くだけで、
//! 実行はアプリの画面でユーザーが承認してから。

use crate::model::{Issue, ItemReport, Kind, Report};
use crate::store::Store;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub const PROTOCOL: &str = "2026-07-28";
pub const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const META_VER: &str = "io.modelcontextprotocol/protocolVersion";
const META_CAPS: &str = "io.modelcontextprotocol/clientCapabilities";

const INSTRUCTIONS: &str = "vaultsweep: 1Password の診断結果（漏洩・使い回し・弱い・事故サイト・2段階未設定・使ってなさそう・重複）を読めます。\
パスワード・ユーザー名・メモ欄は返しません。パスワード変更と保管庫への移動は依頼を置くだけで、ユーザーがアプリで承認して実行します。\
Reads vaultsweep audit results. Never returns passwords, usernames or notes. Changes and archiving are only requested; the user approves them in the app.";

/// 診断を走らせる口（本物は op を呼ぶ。試験ではにせもの）
pub trait Scanner {
    fn scan(&self, store: &Store) -> Result<Report, String>;
}

pub struct Server<'a> {
    pub store: &'a Store,
    pub scanner: &'a dyn Scanner,
    legacy: bool,
}

fn err(id: &Value, code: i64, msg: &str, data: Option<Value>) -> Value {
    let mut e = json!({ "code": code, "message": msg });
    if let Some(d) = data {
        e["data"] = d;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": e })
}

fn ok(id: &Value, mut result: Value, legacy: bool) -> Value {
    if !legacy {
        result["resultType"] = json!("complete");
    }
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn server_info() -> Value {
    json!({ "name": "vaultsweep", "version": env!("CARGO_PKG_VERSION") })
}

pub fn tools() -> Value {
    let kinds: Vec<&str> = Kind::ALL.iter().map(|k| k.as_str()).collect();
    let id_schema = json!({ "type": "object", "properties": { "id": { "type": "string", "description": "項目ID（list_issues で得たもの）" } }, "required": ["id"], "additionalProperties": false });
    json!([
        { "name": "get_summary", "title": "診断のまとめ",
          "description": "最後の診断の安全度・項目数・問題の種類ごとの件数・解約リストの件数を返す。",
          "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
          "annotations": { "readOnlyHint": true } },
        { "name": "list_issues", "title": "問題のある項目",
          "description": "問題のある項目を危ない順に返す。kind で種類をしぼれる。パスワードやユーザー名は含まない。",
          "inputSchema": { "type": "object", "properties": {
              "kind": { "type": "string", "enum": kinds },
              "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 20 } }, "additionalProperties": false },
          "annotations": { "readOnlyHint": true } },
        { "name": "get_item_issues", "title": "1件の詳しい問題",
          "description": "サイト名の一部か項目IDで探し、問題・退会のしかた・2段階認証の説明ページを返す。",
          "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "query": { "type": "string" } }, "additionalProperties": false },
          "annotations": { "readOnlyHint": true } },
        { "name": "run_scan", "title": "今すぐ診断",
          "description": "1Password を読み直して診断する。1Password がロック中なら、パソコンに承認の画面が出る。",
          "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
          "annotations": { "readOnlyHint": false, "destructiveHint": false, "openWorldHint": true } },
        { "name": "mark_in_use", "title": "使っている印",
          "description": "「使っている」印を付ける（または外す）。付けると「使ってなさそう」が出なくなる。1Password の中身は変えない。",
          "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "in_use": { "type": "boolean", "default": true } }, "required": ["id"], "additionalProperties": false },
          "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true } },
        { "name": "add_to_cancel_list", "title": "解約リストに入れる",
          "description": "解約リストに入れる。退会そのものはユーザーが行う。",
          "inputSchema": id_schema.clone(),
          "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true } },
        { "name": "request_password_change", "title": "パスワード変更を依頼",
          "description": "パスワード変更の依頼をアプリに置く。ユーザーがアプリで承認して進める。ここでは何も変えない。",
          "inputSchema": id_schema.clone(),
          "annotations": { "readOnlyHint": false, "destructiveHint": false } },
        { "name": "request_archive", "title": "保管庫への移動を依頼",
          "description": "1Password の保管庫（アーカイブ）へ移す依頼をアプリに置く。ユーザーがアプリで承認するまで実行しない。完全には消さない。",
          "inputSchema": id_schema,
          "annotations": { "readOnlyHint": false, "destructiveHint": false } }
    ])
}

fn issue_json(i: &Issue) -> Value {
    // 秘密でない情報だけ
    match i {
        Issue::Leak { count } => json!({ "kind": "leak", "seen_in_breaches": count }),
        Issue::Breach { breach, date } => {
            json!({ "kind": "breach", "breach": breach, "date": date })
        }
        Issue::Reuse { others, total, .. } => {
            json!({ "kind": "reuse", "same_as": others, "same_count": total })
        }
        Issue::Weak { strength } => json!({ "kind": "weak", "strength": strength }),
        Issue::Nofa { doc } => json!({ "kind": "nofa", "doc": doc }),
        Issue::Stale { days } => json!({ "kind": "stale", "days_since_update": days }),
        Issue::Dup { other, .. } => json!({ "kind": "dup", "duplicate_of": other }),
    }
}

fn item_json(it: &ItemReport, detail: bool) -> Value {
    let mut v = json!({
        "id": it.id, "title": it.title, "domain": it.domain, "risk": it.risk,
        "issues": it.issues.iter().map(issue_json).collect::<Vec<_>>(),
    });
    if detail {
        v["vault"] = json!(it.vault);
        v["days_since_update"] = json!(it.days_since_update);
        v["has_2fa"] = json!(it.has_totp);
        v["passkey_supported"] = json!(it.passkey_site);
        v["twofa_doc"] = json!(it.twofa_doc);
        v["change_password_url"] = json!(
            it.domain
                .as_ref()
                .map(|d| format!("https://{d}/.well-known/change-password"))
        );
        v["deletion"] = json!(it.deletion.as_ref().map(
            |d| json!({ "url": d.url, "difficulty": d.difficulty, "by_email": d.email.is_some() })
        ));
    }
    v
}

impl<'a> Server<'a> {
    pub fn new(store: &'a Store, scanner: &'a dyn Scanner) -> Self {
        Server {
            store,
            scanner,
            legacy: false,
        }
    }

    fn summary(&self) -> Value {
        let st = self.store.state();
        match self.store.report() {
            Some(r) => json!({
                "scanned_at": r.scanned_at, "total": r.total, "with_issues": r.flagged_count(), "score": r.score,
                "counts": r.counts.iter().map(|(k, v)| (k.as_str().to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
                "cancel_list": st.cancel.values().filter(|c| c.state != "archived").count(),
                "pending_requests": st.pending.len(),
                "warnings": r.warnings,
            }),
            None => {
                json!({ "scanned_at": null, "message": "まだ診断していません。run_scan を使ってください。 / Not scanned yet. Call run_scan." })
            }
        }
    }

    fn find(&self, id: &str) -> Result<ItemReport, String> {
        self.store
            .report()
            .and_then(|r| r.items.into_iter().find(|i| i.id == id))
            .ok_or_else(|| format!("項目が見つかりません / item not found: {id}"))
    }

    fn call(&self, name: &str, args: &Value) -> Result<Value, String> {
        let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        match name {
            "get_summary" => Ok(self.summary()),
            "list_issues" => {
                let kind = match s("kind") {
                    Some(k) => Some(Kind::parse(&k).ok_or("unknown kind")?),
                    None => None,
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20)
                    .clamp(1, 200) as usize;
                let r = self
                    .store
                    .report()
                    .ok_or("まだ診断していません / not scanned yet")?;
                let list: Vec<&ItemReport> = r
                    .flagged()
                    .filter(|i| kind.map(|k| i.has(k)).unwrap_or(true))
                    .collect();
                Ok(
                    json!({ "total": list.len(), "items": list.iter().take(limit).map(|i| item_json(i, false)).collect::<Vec<_>>() }),
                )
            }
            "get_item_issues" => {
                let r = self
                    .store
                    .report()
                    .ok_or("まだ診断していません / not scanned yet")?;
                let found: Vec<Value> = if let Some(id) = s("id") {
                    r.items
                        .iter()
                        .filter(|i| i.id == id)
                        .map(|i| item_json(i, true))
                        .collect()
                } else if let Some(q) = s("query")
                    .map(|q| q.to_lowercase())
                    .filter(|q| !q.is_empty())
                {
                    r.items
                        .iter()
                        .filter(|i| {
                            i.title.to_lowercase().contains(&q)
                                || i.domain.as_deref().unwrap_or("").contains(&q)
                        })
                        .take(5)
                        .map(|i| item_json(i, true))
                        .collect()
                } else {
                    return Err("id か query を指定してください / give id or query".into());
                };
                Ok(json!({ "items": found }))
            }
            "run_scan" => {
                self.scanner.scan(self.store)?;
                Ok(self.summary())
            }
            "mark_in_use" => {
                let id = s("id").ok_or("id")?;
                self.find(&id)?;
                let on = args.get("in_use").and_then(|v| v.as_bool()).unwrap_or(true);
                self.store.set_in_use(&id, on)?;
                Ok(json!({ "ok": true, "id": id, "in_use": on }))
            }
            "add_to_cancel_list" => {
                let id = s("id").ok_or("id")?;
                self.find(&id)?;
                self.store.cancel_add(&id)?;
                Ok(
                    json!({ "ok": true, "id": id, "note": "解約リストに入れました。退会はユーザーがアプリの手順で行います。" }),
                )
            }
            "request_password_change" | "request_archive" => {
                let id = s("id").ok_or("id")?;
                let it = self.find(&id)?;
                let kind = if name == "request_archive" {
                    "archive"
                } else {
                    "change"
                };
                let p = self.store.add_pending(kind, &id, &it.title, "mcp")?;
                Ok(
                    json!({ "ok": true, "request_id": p.id, "status": "waiting_for_user",
                    "note": "依頼を置きました。ユーザーが vaultsweep の画面で承認するまで、何も変わりません。 / Waiting for the user to approve in the app." }),
                )
            }
            _ => Err(format!("unknown tool: {name}")),
        }
    }

    fn tool_result(&self, name: &str, args: &Value) -> Value {
        match self.call(name, args) {
            Ok(v) => {
                json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap() }], "structuredContent": v, "isError": false })
            }
            Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
        }
    }

    /// 1通を処理する。通知（id なし）なら None。
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let Some(id) = id else {
            return None; // 通知は返事しない
        };
        if method.is_empty() {
            return Some(err(&id, -32600, "Invalid Request", None));
        }
        // 古い版
        if method == "initialize" {
            self.legacy = true;
            let asked = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ver = if LEGACY.contains(&asked) {
                asked
            } else {
                LEGACY[0]
            };
            return Some(ok(
                &id,
                json!({ "protocolVersion": ver, "capabilities": { "tools": { "listChanged": false } }, "serverInfo": server_info(), "instructions": INSTRUCTIONS }),
                true,
            ));
        }
        let meta = params.get("_meta");
        let ver = meta.and_then(|m| m.get(META_VER)).and_then(|v| v.as_str());
        let new_era = ver.is_some() || method == "server/discover";
        if method == "server/discover" {
            return Some(ok(
                &id,
                json!({ "supportedVersions": [PROTOCOL], "capabilities": { "tools": {} },
                        "_meta": { "io.modelcontextprotocol/serverInfo": server_info() }, "instructions": INSTRUCTIONS }),
                false,
            ));
        }
        if new_era {
            let v = ver.unwrap_or("");
            if v != PROTOCOL {
                return Some(err(
                    &id,
                    -32022,
                    "Unsupported protocol version",
                    Some(json!({ "supported": [PROTOCOL], "requested": v })),
                ));
            }
            if meta.and_then(|m| m.get(META_CAPS)).is_none() {
                return Some(err(&id, -32602, "Missing _meta clientCapabilities", None));
            }
        } else if !self.legacy && method != "ping" {
            return Some(err(&id, -32602, "Missing _meta protocolVersion", None));
        }
        let legacy = !new_era;
        Some(match method {
            "ping" => ok(&id, json!({}), legacy),
            "tools/list" => ok(&id, json!({ "tools": tools() }), legacy),
            "tools/call" => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if !tools()
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|t| t["name"] == name)
                {
                    return Some(err(&id, -32602, &format!("Unknown tool: {name}"), None));
                }
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                ok(&id, self.tool_result(name, &args), legacy)
            }
            _ => err(&id, -32601, "Method not found", None),
        })
    }

    /// 標準入出力で動かす
    pub fn serve(&mut self, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(&line) {
                Ok(Value::Array(_)) => Some(err(&Value::Null, -32600, "Batch not supported", None)),
                Ok(v) => self.handle(&v),
                Err(_) => Some(err(&Value::Null, -32700, "Parse error", None)),
            };
            if let Some(r) = reply {
                writeln!(output, "{}", serde_json::to_string(&r).unwrap())?;
                output.flush()?;
            }
        }
        Ok(())
    }
}
