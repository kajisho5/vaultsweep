# vaultsweep

[English](README.md) · **日本語**

1Password を診断して、危ないパスワードを危ない順に並べ、パスワードの変更と、使っていないサービスの解約を1件ずつ手伝う無料のアプリです。Windows と Mac で動き、中身はすべて公開しています（オープンソース）。

[紹介ページ](https://kajisho5.github.io/vaultsweep/ja/) · [ダウンロード](https://github.com/kajisho5/vaultsweep/releases/latest) · [応援する](https://github.com/sponsors/kajisho5)

> 1Password 公式のアプリではありません。1Password 公式の CLI（コマンドで操作する道具）を使い、あなたのパソコンの中だけで動きます。

## できること

- **8種類の問題を見つける**：漏洩したパスワード、事故があったサイト、使い回し、似ているパスワード（1〜2文字だけの違い）、弱いパスワード、2段階認証が未設定、長い間さわっていない項目、重複。漏洩から順に並べます。
- **パスワードの変更を手伝う**：1件ずつでも、まとめてでも変えられます。新しいパスワードは 1Password の中で作って保存します。サイトで変えられなかったら、ボタン1つで元に戻せます。
- **サイトごとのパスワードの決まりに合わせる**：「記号が使えない」「20文字まで」「数字だけ」といった決まりを、Apple が公開しているデータ（約430サイト）から読みます。作ったパスワードが決まりに合うかも確かめます。サイトごとに自分で決めることもできます。
- **解約を手伝う**：個人情報をダミーに置き換えてから退会し、記録を残して、1Password の保管庫（アーカイブ）へ移します。
- **自動で診断する**：毎日・毎週・毎月の時刻を決めておくと裏で診断し、新しい問題だけを知らせます。
- **AI アプリから聞ける（MCP：AI アプリとつなぐ口）**：Claude などに「どのパスワードが漏れてる？」と聞けます。AI に渡すのは点数・件数・サイト名だけで、パスワードは渡しません。
- **自動で更新する**：新しい版は、署名を確かめてから入れます。
- 日本語・英語、ライト・ダークに対応しています。

## 安全のための決まり

- パスワード・ユーザー名・メモは、画面にも、保存するファイルにも、AI にも出しません。
- 新しいパスワードは `op`（1Password CLI）に作らせます。元に戻すときも、値はコマンドの引数に入れず、標準入力で渡します。
- 漏洩の確認で外に送るのは、パスワードの SHA-1 ハッシュ（指紋のようなもの）の先頭5文字だけです（Have I Been Pwned の k-匿名性）。返ってくる件数から推測されないよう、Add-Padding も付けています。
- コピーしたパスワードは30秒で消えます。Windows ではクリップボードの履歴とクラウドへの同期に残らないように、Mac では「隠すべき内容」の印を付けます。
- 完全には消しません。片付けは 1Password の保管庫へ移すだけなので、いつでも戻せます。
- AI アプリからの「変更」「片付け」は依頼として置くだけです。アプリの画面であなたが承認するまで、何も起きません。
- 外に問い合わせるのは、下の「データの出典」と、新しい版の確認（GitHub）だけです。「オフライン」の設定にすれば、どれも止まります。

## 使い始める

1. 1Password アプリを開いてロックを解除し、Windows Hello か Touch ID をオンにする
2. 1Password の「設定」→「開発者」→「1Password CLI と連携（Integrate with 1Password CLI）」をオンにする
3. 1Password CLI を入れる（[公式の手順](https://www.1password.dev/cli/get-started)）
   - Windows（PowerShell）：`winget install 1password-cli`
   - Mac（Homebrew）：`brew install 1password-cli`
4. [Releases](https://github.com/kajisho5/vaultsweep/releases/latest) からダウンロードする
   - Windows 10・11：`vaultsweep_<版>_x64-setup.exe`
   - Mac（Apple シリコン・Intel、macOS 11 以降）：`vaultsweep-<版>.dmg`。Apple の正式な署名（Developer ID）と公証を受けています
5. アプリの「今すぐ診断」を押し、1Password の確認が出たら許可する

Windows 版はまだ署名をしていないため、初回は SmartScreen が「PC を保護しました」と出ることがあります。「詳細情報」→「実行」で進めます。

## AI アプリから使う（MCP）

アプリの設定画面に、AI アプリへ貼り付ける設定が出ます。形は次のとおりです（`command` は実際の場所に置き換わります）。

```json
{ "mcpServers": { "vaultsweep": { "command": "C:\\...\\vaultsweep-mcp.exe", "args": ["--mcp"] } } }
```

Mac では、アプリの中にあります：`/Applications/vaultsweep.app/Contents/MacOS/vaultsweep-mcp`

使える道具：`get_summary` `list_issues` `get_item_issues` `run_scan` `mark_in_use` `add_to_cancel_list` `request_password_change` `request_archive`。MCP の新しい版（2026-07-28、`server/discover`）と、`initialize` から始まる古い版の両方に答えます。

## データの出典

| 出典 | 使い道 | ライセンス |
| --- | --- | --- |
| [Have I Been Pwned](https://haveibeenpwned.com/) | 漏洩パスワード・事故サイト（メールの確認は有料のキーが要る） | CC BY 4.0 |
| [JustDeleteMe](https://github.com/jdm-contrib/jdm) | 退会ページと、退会の難しさ | MIT |
| [2FA Directory](https://2fa.directory/) | 2段階認証に対応しているサイト | MIT |
| [Passkeys Directory](https://passkeys.2fa.directory/) | パスキーに対応しているサイト | CC BY 4.0 |
| [Apple Password Manager Resources](https://github.com/apple/password-manager-resources) | サイトごとのパスワードの決まり（アプリにも控えを同梱） | MIT |

Data sourced from 2FA Directory by 2factorauth.

## 作り

| 部分 | 中身 |
| --- | --- |
| `crates/core` | 中身（Rust）：1Password CLI との連携、判定、HIBP、公開されている一覧、パスワードの決まり、保存、MCP サーバー、`vaultsweep-mcp` |
| `src-tauri` | アプリの枠（Tauri 2）：クリップボード、通知、自動診断（タスク スケジューラ／launchd）、更新 |
| `src` | 画面（React・TypeScript・Tailwind CSS）。アプリの外で開くと、架空のデータで動くデモになります |
| `site` | 紹介ページ（Astro）。GitHub Pages で公開しています |

保存場所：Windows は `%APPDATA%\vaultsweep`、Mac は `~/Library/Application Support/vaultsweep`。設定・解約リスト・履歴・最後の診断結果だけで、秘密の値は入りません。

### 開発

```sh
npm ci
cargo test -p vaultsweep-core     # 中身の試験
npm run dev                        # 画面だけ（デモ）を http://localhost:5173 で開く
```

ビルドは GitHub Actions（`.github/workflows/build.yml`）で行います。`package.json` の版番号のリリースがまだ無ければ、`main` に反映したときに Windows 版と Mac 版を作り、Mac 版に署名と公証をして、更新用のファイルと一緒にリリースを作ります。

## 応援

vaultsweep は広告も有料プランもなしで続けています。気に入ったら [GitHub Sponsors で応援](https://github.com/sponsors/kajisho5) してもらえると、次の機能や保守に使います。お金を払わなくても、全部の機能を使えます。

## ライセンス

MIT。1Password は AgileBits Inc. の商標です。
