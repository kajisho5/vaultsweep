//! にせものの op と外部データで、診断〜変更〜MCP までを通して確かめる。
//! データはすべて架空。

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;
use vaultsweep_core::hibp::sha1_hex;
use vaultsweep_core::mcp::{Scanner, Server};
use vaultsweep_core::model::{Kind, Report};
use vaultsweep_core::net::Fetcher;
use vaultsweep_core::op::{self, OpError, OpResult, OpRunner};
use vaultsweep_core::scan;
use vaultsweep_core::store::Store;
use zeroize::Zeroizing;

const SECRETS: &[&str] = &[
    "password1",
    "Zq8$Lr!vT2#pW9@kM6&n",
    "Yh3%Jd!sQ7#bN4@cX1&m",
    "Wb5^Ke!uR8#fG2@hP7&q",
    "taro.sample@example.com",
];

struct FakeOp {
    items: Mutex<Vec<Value>>,
    calls: Mutex<Vec<(Vec<String>, Option<String>)>>,
}

fn login(
    id: &str,
    title: &str,
    url: &str,
    pw: &str,
    user: &str,
    updated: &str,
    otp: bool,
) -> Value {
    let mut fields = vec![
        json!({"id":"username","type":"STRING","purpose":"USERNAME","label":"username","value":user}),
        json!({"id":"password","type":"CONCEALED","purpose":"PASSWORD","label":"password","value":pw}),
        json!({"id":"notesPlain","type":"STRING","purpose":"NOTES","label":"notesPlain","value":"秘密のメモ"}),
    ];
    if otp {
        fields.push(json!({"id":"otp1","type":"OTP","label":"one-time password","value":"otpauth://totp/x?secret=ABC","totp":"123456"}));
    }
    json!({"id":id,"title":title,"vault":{"id":"v1","name":"Private"},"category":"LOGIN",
           "urls":[{"primary":true,"href":url}],"updated_at":updated,"fields":fields})
}

impl FakeOp {
    fn new() -> Self {
        FakeOp {
            items: Mutex::new(vec![
                login(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "サンプル通販A",
                    "https://www.shop-a.example/login",
                    "password1",
                    "taro.sample@example.com",
                    "2020-01-01T00:00:00Z",
                    false,
                ),
                login(
                    "bbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "架空SNS-B",
                    "https://sns-b.example",
                    "password1",
                    "sample_b",
                    "2026-10-01T00:00:00Z",
                    false,
                ),
                login(
                    "cccccccccccccccccccccccccc",
                    "ダミー銀行（架空）",
                    "https://bank-d.example",
                    "Zq8$Lr!vT2#pW9@kM6&n",
                    "sample_c",
                    "2026-10-01T00:00:00Z",
                    true,
                ),
                login(
                    "dddddddddddddddddddddddddd",
                    "テスト求人H",
                    "https://jobs-h.example",
                    "Yh3%Jd!sQ7#bN4@cX1&m",
                    "u1",
                    "2026-10-02T00:00:00Z",
                    false,
                ),
                login(
                    "eeeeeeeeeeeeeeeeeeeeeeeeee",
                    "テスト求人H（古い）",
                    "https://www.jobs-h.example/",
                    "Wb5^Ke!uR8#fG2@hP7&q",
                    "U1",
                    "2026-10-03T00:00:00Z",
                    false,
                ),
            ]),
            calls: Mutex::new(vec![]),
        }
    }
    fn find(&self, id: &str) -> Option<Value> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|v| v["id"] == id)
            .cloned()
    }
}

impl OpRunner for FakeOp {
    fn run(&self, args: &[&str], stdin: Option<&[u8]>) -> OpResult<Zeroizing<Vec<u8>>> {
        self.calls.lock().unwrap().push((
            args.iter().map(|s| s.to_string()).collect(),
            stdin.map(|b| String::from_utf8_lossy(b).to_string()),
        ));
        let out = match args {
            ["--version"] => "2.41.1".to_string(),
            ["vault", "list", ..] => r#"[{"id":"v1","name":"Private"}]"#.to_string(),
            ["whoami", ..] => r#"{"url":"sample.1password.example"}"#.to_string(),
            ["item", "list", ..] => {
                let v: Vec<Value> = self
                    .items
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|i| json!({"id":i["id"],"title":i["title"]}))
                    .collect();
                serde_json::to_string(&v).unwrap()
            }
            ["item", "get", "-", ..] => self
                .items
                .lock()
                .unwrap()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
            ["item", "get", id, ..] => self
                .find(id)
                .ok_or(OpError::Other("not found".into()))?
                .to_string(),
            ["item", "edit", id, flag, ..] if flag.starts_with("--generate-password=") => {
                let mut items = self.items.lock().unwrap();
                let it = items.iter_mut().find(|v| v["id"] == *id).unwrap();
                it["fields"][1]["value"] = json!("Gen-Fresh-Pass-0001!xyz");
                it["updated_at"] = json!("2026-10-25T08:00:00Z");
                "{}".into()
            }
            ["item", "edit", id, "--format", "json"] if stdin.is_some() => {
                let v: Value = serde_json::from_slice(stdin.unwrap()).unwrap();
                let mut items = self.items.lock().unwrap();
                let it = items.iter_mut().find(|x| x["id"] == *id).unwrap();
                *it = v;
                "{}".into()
            }
            ["item", "edit", ..] => "{}".into(),
            ["item", "delete", id, "--archive"] => {
                self.items.lock().unwrap().retain(|v| v["id"] != *id);
                String::new()
            }
            ["read", r] if r.starts_with("op://") => "fake-key".into(),
            _ => return Err(OpError::Other(format!("unexpected {args:?}"))),
        };
        Ok(Zeroizing::new(out.into_bytes()))
    }
}

struct FakeNet;
impl Fetcher for FakeNet {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<(u16, Vec<u8>), String> {
        let leaked = sha1_hex("password1");
        let body = if let Some(p) = url.strip_prefix("https://api.pwnedpasswords.com/range/") {
            assert!(headers.contains(&("Add-Padding", "true")));
            assert_eq!(p.len(), 5, "先頭5文字だけを送る");
            if p == &leaked[..5] {
                format!(
                    "{}:2345\r\n0000000000000000000000000000000000A:0",
                    &leaked[5..]
                )
            } else {
                "00000000000000000000000000000000001:3".into()
            }
        } else if url.ends_with("/v4/totp.json") {
            r#"{"shop-a.example":{"methods":["totp"],"documentation":"https://doc.shop-a.example/2fa"},"sns-b.example":{"methods":["sms","totp"]},"bank-d.example":{"methods":["totp"]}}"#.into()
        } else if url.ends_with("/v1/supported.json") {
            r#"{"bank-d.example":{"mfa":"allowed"}}"#.into()
        } else if url.ends_with("sites.json") {
            r#"[{"name":"Shop A","url":"https://shop-a.example/leave","difficulty":"medium","domains":["shop-a.example"]}]"#.into()
        } else if url.ends_with("/api/v3/breaches") {
            r#"[{"Name":"ShopA","Title":"Shop A","Domain":"shop-a.example","BreachDate":"2021-05-01","AddedDate":"2022-01-01T00:00:00Z","DataClasses":["Passwords"],"IsVerified":true}]"#.into()
        } else {
            return Ok((404, vec![]));
        };
        Ok((200, body.into_bytes()))
    }
}

fn tmp_store(name: &str) -> Store {
    let d = std::env::temp_dir().join(format!("vaultsweep-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    Store::new(d)
}

fn kinds(r: &Report, id: &str) -> Vec<Kind> {
    r.items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .issues
        .iter()
        .map(|i| i.kind())
        .collect()
}

fn assert_no_secrets(s: &str) {
    for x in SECRETS {
        assert!(!s.contains(x), "秘密の値が出ている: {x}");
    }
    assert!(!s.contains("秘密のメモ"));
    assert!(!s.contains(&sha1_hex("password1")), "ハッシュも出さない");
}

#[test]
fn scan_change_restore_archive() {
    let op = FakeOp::new();
    let store = tmp_store("flow");
    let res = scan::run(&op, Some(&FakeNet), &store, &|_| {}).unwrap();
    let r = &res.report;
    assert_eq!(r.total, 5);
    let a = kinds(r, "aaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(
        a,
        vec![
            Kind::Leak,
            Kind::Breach,
            Kind::Reuse,
            Kind::Weak,
            Kind::Nofa,
            Kind::Stale
        ]
    );
    assert_eq!(
        kinds(r, "bbbbbbbbbbbbbbbbbbbbbbbbbb"),
        vec![Kind::Leak, Kind::Reuse, Kind::Weak, Kind::Nofa]
    );
    assert!(kinds(r, "cccccccccccccccccccccccccc").is_empty());
    assert_eq!(kinds(r, "dddddddddddddddddddddddddd"), vec![Kind::Dup]);
    assert_eq!(kinds(r, "eeeeeeeeeeeeeeeeeeeeeeeeee"), vec![Kind::Dup]);
    let ia = r.items.iter().find(|i| i.id.starts_with('a')).unwrap();
    assert_eq!(ia.domain.as_deref(), Some("shop-a.example"));
    assert_eq!(ia.deletion.as_ref().unwrap().difficulty, "medium");
    assert_eq!(ia.username_hint.as_deref(), Some("ta•••"));
    assert!(
        r.items
            .iter()
            .find(|i| i.id.starts_with('c'))
            .unwrap()
            .passkey_site
    );
    assert_eq!(r.items[0].id, "aaaaaaaaaaaaaaaaaaaaaaaaaa", "危ない順");
    assert!(r.score < 100);
    assert_eq!(r.counts[&Kind::Reuse], 2);

    // 保存ファイルに秘密がない
    let saved = std::fs::read_to_string(store.dir.join("report.json")).unwrap();
    assert_no_secrets(&saved);
    assert_no_secrets(&std::fs::read_to_string(store.dir.join("state.json")).unwrap());

    // パスワード変更：今の値を読む → 作る → 確認
    let old = op::read_password(&op, "aaaaaaaaaaaaaaaaaaaaaaaaaa")
        .unwrap()
        .unwrap();
    assert_eq!(old.as_str(), "password1");
    op::generate_password(&op, "aaaaaaaaaaaaaaaaaaaaaaaaaa", &op::recipe(24, true)).unwrap();
    let new = op::read_password(&op, "aaaaaaaaaaaaaaaaaaaaaaaaaa")
        .unwrap()
        .unwrap();
    assert_eq!(new.as_str(), "Gen-Fresh-Pass-0001!xyz");
    // 元に戻す：値は引数ではなく標準入力で渡る
    op::restore_password(&op, "aaaaaaaaaaaaaaaaaaaaaaaaaa", &old).unwrap();
    assert_eq!(
        op::read_password(&op, "aaaaaaaaaaaaaaaaaaaaaaaaaa")
            .unwrap()
            .unwrap()
            .as_str(),
        "password1"
    );
    for (args, _) in op.calls.lock().unwrap().iter() {
        for a in args {
            assert!(
                !a.contains("password1") && !a.contains("Gen-Fresh"),
                "引数に秘密: {a}"
            );
        }
    }
    let edit_stdin = op
        .calls
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(a, s)| a[1] == "edit" && s.is_some())
        .cloned()
        .unwrap();
    assert!(edit_stdin.1.unwrap().contains("password1"));

    // 変えた扱いにすると、使い回しの相手（B）の札も外れる
    let r2 = store
        .mark_changed("aaaaaaaaaaaaaaaaaaaaaaaaaa")
        .unwrap()
        .unwrap();
    assert_eq!(
        kinds(&r2, "aaaaaaaaaaaaaaaaaaaaaaaaaa"),
        vec![Kind::Nofa, Kind::Stale]
    );
    assert_eq!(
        kinds(&r2, "bbbbbbbbbbbbbbbbbbbbbbbbbb"),
        vec![Kind::Leak, Kind::Weak, Kind::Nofa]
    );
    assert!(r2.score > r.score);
    assert_eq!(store.state().fixed.len(), 1);

    // 使っている印
    let r3 = store
        .set_in_use("aaaaaaaaaaaaaaaaaaaaaaaaaa", true)
        .unwrap()
        .unwrap();
    assert_eq!(kinds(&r3, "aaaaaaaaaaaaaaaaaaaaaaaaaa"), vec![Kind::Nofa]);

    // 解約 → 保管庫
    store.cancel_add("eeeeeeeeeeeeeeeeeeeeeeeeee").unwrap();
    store
        .cancel_set("eeeeeeeeeeeeeeeeeeeeeeeeee", "left", Some("R-000".into()))
        .unwrap();
    op::add_text_fields(
        &op,
        "eeeeeeeeeeeeeeeeeeeeeeeeee",
        &[("退会日", "2026-10-25"), ("受付番号", "R-000")],
    )
    .unwrap();
    op::archive(&op, "eeeeeeeeeeeeeeeeeeeeeeeeee").unwrap();
    store
        .cancel_set("eeeeeeeeeeeeeeeeeeeeeeeeee", "archived", None)
        .unwrap();
    store.remove_item("eeeeeeeeeeeeeeeeeeeeeeeeee").unwrap();
    assert_eq!(store.report().unwrap().total, 4);
    assert_eq!(
        store.state().cancel["eeeeeeeeeeeeeeeeeeeeeeeeee"].state,
        "archived"
    );
    let del = op
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(a, _)| a == &["item", "delete", "eeeeeeeeeeeeeeeeeeeeeeeeee", "--archive"]);
    assert!(del, "完全削除ではなく --archive");

    // 2回目の診断：新しい問題の数
    let res2 = scan::run(&op, Some(&FakeNet), &store, &|_| {}).unwrap();
    assert_eq!(res2.new_issues, 0);
    assert_eq!(store.state().history.len(), 2);
}

#[test]
fn bad_ids_rejected() {
    let op = FakeOp::new();
    assert_eq!(op::archive(&op, "--all").unwrap_err(), OpError::BadId);
    assert_eq!(
        op::generate_password(&op, "a b", "letters,24").unwrap_err(),
        OpError::BadId
    );
    assert!(op.calls.lock().unwrap().is_empty());
}

#[test]
fn offline_scan_sends_nothing() {
    struct NoNet;
    impl Fetcher for NoNet {
        fn get(&self, url: &str, _: &[(&str, &str)]) -> Result<(u16, Vec<u8>), String> {
            panic!("通信してはいけない: {url}");
        }
    }
    let op = FakeOp::new();
    let store = tmp_store("offline");
    let mut s = store.settings();
    s.offline = true;
    store.save_settings(s).unwrap();
    let r = scan::run(&op, Some(&NoNet), &store, &|_| {})
        .unwrap()
        .report;
    assert!(r.warnings.contains(&"offline".to_string()));
    assert!(!kinds(&r, "aaaaaaaaaaaaaaaaaaaaaaaaaa").contains(&Kind::Leak));
    assert!(kinds(&r, "aaaaaaaaaaaaaaaaaaaaaaaaaa").contains(&Kind::Reuse));
}

struct FakeScanner;
impl Scanner for FakeScanner {
    fn scan(&self, store: &Store) -> Result<Report, String> {
        scan::run(&FakeOp::new(), Some(&FakeNet), store, &|_| {})
            .map(|r| r.report)
            .map_err(|e| e.to_string())
    }
}

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}})
}

fn run_lines(store: &Store, lines: &[Value]) -> Vec<Value> {
    let input: String = lines.iter().map(|l| l.to_string() + "\n").collect();
    let mut out = vec![];
    Server::new(store, &FakeScanner)
        .serve(input.as_bytes(), &mut out)
        .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_no_secrets(&text);
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn mcp_new_era() {
    let store = tmp_store("mcp");
    let out = run_lines(
        &store,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"_meta":meta(),"name":"get_summary","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"_meta":meta(),"name":"run_scan","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"_meta":meta(),"name":"list_issues","arguments":{"kind":"reuse"}}}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"_meta":meta(),"name":"get_item_issues","arguments":{"query":"通販"}}}),
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"_meta":meta(),"name":"request_archive","arguments":{"id":"bbbbbbbbbbbbbbbbbbbbbbbbbb"}}}),
            json!({"jsonrpc":"2.0","id":8,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2099-01-01","io.modelcontextprotocol/clientCapabilities":{}}}}),
            json!({"jsonrpc":"2.0","id":9,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}),
            json!({"jsonrpc":"2.0","id":10,"method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","method":"notifications/whatever"}),
            json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"_meta":meta(),"name":"nope","arguments":{}}}),
        ],
    );
    assert_eq!(out.len(), 11, "通知には返事しない");
    assert_eq!(out[0]["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(out[0]["result"]["resultType"], "complete");
    assert_eq!(
        out[0]["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "vaultsweep"
    );
    assert_eq!(out[1]["result"]["tools"].as_array().unwrap().len(), 8);
    assert_eq!(
        out[2]["result"]["structuredContent"]["scanned_at"],
        Value::Null
    );
    assert_eq!(out[3]["result"]["structuredContent"]["total"], 5);
    assert_eq!(out[4]["result"]["structuredContent"]["total"], 2);
    let item = &out[5]["result"]["structuredContent"]["items"][0];
    assert_eq!(
        item["change_password_url"],
        "https://shop-a.example/.well-known/change-password"
    );
    assert!(item.get("username_hint").is_none());
    assert_eq!(
        out[6]["result"]["structuredContent"]["status"],
        "waiting_for_user"
    );
    assert_eq!(store.state().pending.len(), 1);
    assert_eq!(
        store.report().unwrap().total,
        5,
        "依頼だけで、まだ片付けない"
    );
    assert_eq!(out[7]["error"]["code"], -32022);
    assert_eq!(out[7]["error"]["data"]["requested"], "2099-01-01");
    assert_eq!(out[8]["error"]["code"], -32602);
    assert_eq!(out[9]["error"]["code"], -32602);
    assert_eq!(out[10]["error"]["code"], -32602);
}

#[test]
fn mcp_legacy() {
    let store = tmp_store("mcp-legacy");
    let out = run_lines(
        &store,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"x","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_scan","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_issues","arguments":{"limit":1}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"mark_in_use","arguments":{"id":"zzzz"}}}),
        ],
    );
    assert_eq!(out[0]["result"]["protocolVersion"], "2025-06-18");
    assert!(out[0]["result"].get("resultType").is_none());
    assert_eq!(out[1]["result"]["tools"].as_array().unwrap().len(), 8);
    assert_eq!(
        out[3]["result"]["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(out[4]["result"]["isError"], true);
    let _ = HashMap::<u8, u8>::new();
}

#[test]
fn concurrent_state_updates_are_not_lost() {
    let store = std::sync::Arc::new(tmp_store("concurrent"));
    let mut hs = vec![];
    for t in 0..8 {
        let st = store.clone();
        hs.push(std::thread::spawn(move || {
            for i in 0..100 {
                st.update_state(|s| {
                    s.in_use.insert(format!("t{t}i{i}"));
                })
                .unwrap();
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    assert_eq!(store.state().in_use.len(), 800, "同時に書いても消えない");
}

#[test]
fn broken_state_file_is_kept_aside() {
    let store = tmp_store("broken");
    std::fs::write(store.dir.join("state.json"), b"{not json").unwrap();
    store.update_state(|s| s.in_use.insert("x".into())).unwrap();
    let kept = std::fs::read_dir(&store.dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("state.json.broken-")
        });
    assert!(kept, "壊れたファイルは消さずに取っておく");
    std::fs::write(store.dir.join("settings.json"), b"{oops").unwrap();
    assert!(
        store.settings().offline,
        "設定が読めないときは外へ問い合わせない"
    );
}

#[test]
fn url_secrets_are_removed() {
    use vaultsweep_core::audit::clean_url;
    assert_eq!(
        clean_url("https://admin:secret@192.168.1.1/x?token=abc#f").as_deref(),
        Some("https://192.168.1.1/x")
    );
    let op = FakeOp::new();
    op.items.lock().unwrap()[1]["urls"] =
        json!([{"primary":true,"href":"https://sample_b:hunter2@sns-b.example/home?session=zzz"}]);
    let store = tmp_store("urlsecret");
    scan::run(&op, Some(&FakeNet), &store, &|_| {}).unwrap();
    let saved = std::fs::read_to_string(store.dir.join("report.json")).unwrap();
    assert!(!saved.contains("hunter2") && !saved.contains("session=zzz"));
}

#[test]
fn one_unreadable_item_does_not_stop_scan() {
    struct Flaky(FakeOp);
    impl OpRunner for Flaky {
        fn run(&self, args: &[&str], stdin: Option<&[u8]>) -> OpResult<Zeroizing<Vec<u8>>> {
            if args.get(2) == Some(&"-") || args.get(2) == Some(&"dddddddddddddddddddddddddd") {
                return Err(OpError::Other("permission".into()));
            }
            self.0.run(args, stdin)
        }
    }
    let store = tmp_store("flaky");
    let r = scan::run(&Flaky(FakeOp::new()), Some(&FakeNet), &store, &|_| {})
        .unwrap()
        .report;
    assert_eq!(r.total, 4);
    assert!(r.warnings.contains(&"skipped:1".to_string()));
}

#[test]
fn similar_passwords_detected_and_cleared() {
    let op = FakeOp::new();
    {
        let mut it = op.items.lock().unwrap();
        it[3]["fields"][1]["value"] = json!("Sample-Kumo2023!");
        it[4]["fields"][1]["value"] = json!("Sample-Kumo2024!");
    }
    let store = tmp_store("similar");
    let r = scan::run(&op, Some(&FakeNet), &store, &|_| {})
        .unwrap()
        .report;
    assert!(kinds(&r, "dddddddddddddddddddddddddd").contains(&Kind::Similar));
    assert!(kinds(&r, "eeeeeeeeeeeeeeeeeeeeeeeeee").contains(&Kind::Similar));
    // 同じパスワード（A と B）は「使い回し」で、「似ている」には入れない
    assert!(!kinds(&r, "aaaaaaaaaaaaaaaaaaaaaaaaaa").contains(&Kind::Similar));
    let saved = std::fs::read_to_string(store.dir.join("report.json")).unwrap();
    assert!(
        !saved.contains("Kumo") && !saved.to_lowercase().contains("kumo"),
        "パスワードの一部も残さない"
    );
    let r2 = store
        .mark_changed("dddddddddddddddddddddddddd")
        .unwrap()
        .unwrap();
    assert!(!kinds(&r2, "dddddddddddddddddddddddddd").contains(&Kind::Similar));
    assert!(
        !kinds(&r2, "eeeeeeeeeeeeeeeeeeeeeeeeee").contains(&Kind::Similar),
        "相手が1件だけになったら外す"
    );
}
