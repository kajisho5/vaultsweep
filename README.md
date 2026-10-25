# vaultsweep

1Password の中を診断して、パスワードの変更や使っていないサービスの解約を手伝う Windows アプリです。
（English summary is at the bottom.）

- **見つけるもの**：漏洩したパスワード、事故があったサイト、使い回し、弱いパスワード、2段階認証が未設定、長い間さわっていない項目、重複
- **手伝うこと**：パスワード変更（1件ずつ・まとめて）、解約（個人情報をダミーに置き換えてから退会 → 記録を残す → 1Password の保管庫へ移す）
- **そのほか**：自動診断と通知、安全度の移り変わり、MCP（AI アプリから質問できる）、日本語／英語、ライト／ダーク

## 安全のための決まり

- パスワード・ユーザー名・メモ欄は、画面にも、保存ファイルにも、AI（MCP）にも出しません。
- パスワードは `op`（1Password CLI）に作らせます。元に戻すときも、値はコマンドの引数ではなく標準入力で渡します。
- 漏洩の確認は、パスワードの SHA-1 の先頭5文字だけを送る方式（Have I Been Pwned の k-匿名性）です。件数も隠す設定（Add-Padding）を付けています。
- コピーしたパスワードは30秒で消します。Windows のクリップボード履歴とクラウド同期には残らない印を付けます。
- 削除はしません。片付けは 1Password の「保管庫（アーカイブ）」への移動だけで、いつでも戻せます。
- AI からの「変更」「片付け」は依頼として置くだけです。アプリの画面で承認するまで何も起きません。
- 外部に問い合わせるのは決めた5か所だけです（下の「データの出典」）。オフライン診断にもできます。

## 使い始める

1. 1Password アプリを開いてロックを解除し、Windows Hello をオンにする
2. 1Password の「設定」→「開発者」→「1Password CLI と連携（Integrate with 1Password CLI）」をオンにする
3. 1Password CLI を入れる：PowerShell で `winget install 1password-cli`
   （公式の手順：https://www.1password.dev/cli/get-started ）
4. このリポジトリの「Releases」から `vaultsweep_..._x64-setup.exe` をダウンロードして実行する
5. アプリの「今すぐ診断」を押す（1Password の確認が出たら許可）

署名をしていないため、初回は Windows の SmartScreen が「PC を保護しました」と出ることがあります。「詳細情報」→「実行」で進めます。

## MCP（AI アプリから使う）

設定画面に、AI アプリへ貼り付ける設定が出ます。形は次のとおりです（`command` は実際の場所に置き換わります）。

```json
{ "mcpServers": { "vaultsweep": { "command": "C:\\...\\vaultsweep-mcp.exe", "args": ["--mcp"] } } }
```

使える道具：`get_summary` `list_issues` `get_item_issues` `run_scan` `mark_in_use` `add_to_cancel_list` `request_password_change` `request_archive`。
MCP の新しい版（2026-07-28、`server/discover`）と、古い版（`initialize` から始まるもの）の両方に答えます。

## データの出典

| 出典 | 使い道 | ライセンス |
| --- | --- | --- |
| [Have I Been Pwned](https://haveibeenpwned.com/) | 漏洩パスワード・事故サイト（メールの確認は有料キー） | CC BY 4.0 |
| [JustDeleteMe](https://github.com/jdm-contrib/jdm) | 退会ページと難しさ | MIT |
| [2FA Directory](https://2fa.directory/) | 2段階認証に対応しているサイト | MIT |
| [Passkeys Directory](https://passkeys.2fa.directory/) | パスキーに対応しているサイト | CC BY 4.0 |

Data sourced from 2FA Directory by 2factorauth.

## 作り

| 部分 | 中身 |
| --- | --- |
| `crates/core` | 中身（Rust）：1Password CLI 連携、判定、HIBP、一覧データ、保存、MCP サーバー、`vaultsweep-mcp` |
| `src-tauri` | アプリの枠（Tauri 2）：画面とのやりとり、クリップボード、通知、自動診断（タスク スケジューラ） |
| `src` | 画面（React + TypeScript + Tailwind CSS）。アプリの外で開くと、架空のデータで動くデモになります |

保存場所：`%APPDATA%\vaultsweep`（設定・解約リスト・履歴・最後の診断結果。秘密の値は入りません）

### 開発

```sh
npm ci
cargo test -p vaultsweep-core     # 中身の試験
npm run dev                        # 画面だけ（デモ）を http://localhost:5173 で開く
```

アプリ全体のビルドは GitHub Actions（`.github/workflows/build.yml`）の Windows ランナーで行います。

---

## English

vaultsweep is a Windows app that audits your 1Password logins (leaked, breached-site, reused, weak, no 2FA, likely unused, duplicates), walks you through password changes one by one or in bulk, and helps you cancel accounts: replace personal details with dummies, leave, keep a record, then move the item to the 1Password Archive. Passwords, usernames and notes are never shown, stored or sent to AI tools. Leak checks use HIBP k-anonymity with padding. Copied passwords are cleared after 30 seconds and excluded from Windows clipboard history. The MCP server (`vaultsweep-mcp.exe --mcp`) answers both the 2026-07-28 protocol and legacy clients, and only *requests* changes, which you approve in the app. Switch language (Japanese/English) and theme (light/dark/system) in Settings.

License: MIT
