# 自動更新・寄付の案内・サイトごとのパスワードの決まり：設計書

対象：vaultsweep 0.2.0（いまの公開版は 0.1.0）
完成デザイン：[update-sponsor-design.svg](update-sponsor-design.svg)

## 1. やること

1. **自動更新**：アプリが新しい版を見つけて知らせる。ボタン1つで入れ替えて、開き直す。Windows と Mac の両方。
3. **サイトごとのパスワードの決まり**：「記号が使えない」「20文字まで」「数字だけ」などのサイトの決まりに合わせてパスワードを作る（8章）。
2. **寄付の案内**：紹介ページ（英語と日本語）に「応援する」の欄と、フッターのリンクを足す。リンク先は GitHub Sponsors（https://github.com/sponsors/kajisho5、受付中なのを確認済み）。リポジトリの上部に出る「Sponsor」ボタン（.github/FUNDING.yml）も足す。

## 2. 技術の選び方：Sparkle にしない理由

Sparkle は Mac 専用の更新の仕組みで、Windows 版には使えない（公式サイトでも「macOS」とだけ書かれている）。vaultsweep は Tauri で作っていて、Tauri には Windows と Mac の両方に効く公式の更新部品がある。

| | Tauri 公式の更新部品（おすすめ） | Sparkle（Mac）＋ WinSparkle（Windows） | Sparkle（Mac だけ） |
|---|---|---|---|
| 動作の軽さ | ◎ アプリに組み込み。常に動き続ける別のプログラムは無い | ○ | ○ |
| 配布サイズ | ◎ 増えるのは数百KB程度（推測） | △ 仕組みを2つ同梱する | ○ |
| 作りやすさ | ◎ Rust と画面の部品だけで完結する | × Objective-C と C の部品を Tauri につなぐ作業が要る | △ 同じくつなぐ作業が要る |
| 対応 OS | ◎ Windows・Mac（Linux も） | ○ 両方。ただし仕組みが別々 | × Windows は自動更新なし |
| 将来性 | ◎ Tauri 本体と一緒に更新される | ○ | ○ |
| AI での書きやすさ | ◎ 公式の説明と例が多い | △ | △ |

**おすすめ：Tauri 公式の更新部品。** Windows と Mac を1つの仕組みで直せて、アプリの言葉や見た目もそろえやすい。改ざん防止の署名（更新ファイルが本物か確かめる仕組み）があり、新しい版も Apple の署名・公証を受けたまま届く。

使う版（2026-10-26 に確認した最新の安定版）：
- `tauri-plugin-updater` 2.14.0（Rust）／ `@tauri-apps/plugin-updater` 2.14.0（画面側）
- `tauri-plugin-process` 2.4.0（Rust）／ `@tauri-apps/plugin-process` 2.4.0（再起動に使う）
- 3.0.0-alpha も出ているが、試作段階の版なので使わない
- 根拠：https://v2.tauri.app/plugin/updater/ ・ crates.io と npm の登録情報

## 3. 仕組み

1. リリースを作るときに、GitHub Actions が次のものを作ってリリースに置く。
   - Windows：インストーラー（.exe）とその署名（.sig）
   - Mac：アプリをまとめたファイル（.app.tar.gz）とその署名
   - 一覧表 `latest.json`：版番号・各ファイルの場所・署名が書いてある
2. アプリは `https://github.com/kajisho5/vaultsweep/releases/latest/download/latest.json` を読んで、自分より新しい版があるかを見る。
3. 新しい版があれば、概要画面の上に帯を出す（デザイン A）。
4. 「更新して再起動」を押すと、ダウンロードして署名を確かめ、入れ替えて開き直す。

### Mac の注意
- 更新ファイルは、Apple の署名と公証を受けた後のアプリから作る（runway の処理の後で固める）。
- `.app.tar.gz` を作る段階は、ビルドの中ではなく、公証の後の publish の仕事にする。

### 確かめる時と、確かめない時
- 起動して1分後に1回、そのあと6時間ごと。設定の「新しい版を自動で確かめる」で止められる（最初はオン）。
- 設定で「オフライン」がオンなら確かめない。
- 夜の自動診断（画面を出さずに動くとき）では確かめない。

### 押せないとき
パスワード変更の途中（元のパスワードを預かっている間）は「更新して再起動」を押せない。帯の下に1行だけ出す：「変更を確定するか、元に戻してから更新できます」。これまでの「閉じるときの確認」と同じ考え方。

### 送るもの（プライバシー）
GitHub に送るのは、上の一覧表を取りに行く通信だけ。1Password の中身は送らない。プライバシーのページに次の1行を足す（英語と日本語）：
「新しい版の確認のため、GitHub のリリースの一覧表を取りに行きます（設定で止められます）。」

## 4. 画面

- **A 帯**：概要画面の見出しの下。左に青い更新アイコン、題「新しい版 0.2.0 があります」、説明1行。右に「あとで」「更新内容」「更新して再起動」。どれも白い枠のボタンで、琥珀色は使わない（琥珀は「今すぐ診断」だけ）。
  - 押している最中は「ダウンロード中 42%」→「再起動します」に変わる。
  - 失敗したら灰色で「更新できませんでした。あとでもう一度試します」。
- **B 設定**：「その他」の上にカード「更新」。中身は「新しい版を自動で確かめる」のスイッチ、いまの版、最後に確かめた日時、「今すぐ確かめる」ボタン。押した後は、確かめ中・最新・新しい版・つながらない、の4通りの出方がある。
- **C 紹介ページ**：「よくある質問」と「最後の誘い」の間に「応援する」の欄。
  - ハートのアイコン、見出し、2行の説明、「GitHub Sponsors で応援する」ボタン、「お金を払わなくても、全部の機能を使えます」。
  - スマホ幅ではボタンを横いっぱいにする。
  - フッターに「応援する」を足す。
- デモ版（ブラウザで見せる版）では、帯を「0.2.0 があります」の見本として出せるようにする。押しても実際には更新しない。

## 5. 文言

| 場所 | 日本語 | 英語 |
|---|---|---|
| 帯の題 | 新しい版 {v} があります | Version {v} is available |
| 帯の説明 | いまの版は {cur} です。更新すると、アプリが一度閉じて開き直します。 | You have {cur}. Updating closes and reopens the app. |
| ボタン | あとで ／ 更新内容 ／ 更新して再起動 | Later ／ What's new ／ Update and restart |
| 進み具合 | ダウンロード中 {n}% ／ 再起動します | Downloading {n}% ／ Restarting |
| 押せない理由 | 変更を確定するか、元に戻してから更新できます | Confirm or undo the password change first |
| 失敗 | 更新できませんでした。あとでもう一度試します | Couldn't update. We'll try again later |
| 設定の題 | 更新 | Updates |
| 応援の見出し | vaultsweep を応援する | Support vaultsweep |
| 応援の説明 | 広告も有料プランもなしで続けています。気に入ったら GitHub Sponsors で応援してもらえると、次の機能や保守に使います。月ごとでも、1回だけでも大丈夫です。 | No ads, no paid plan. If it helped you, sponsoring on GitHub keeps new features and fixes coming. Monthly or one-time, either is welcome. |
| 応援ボタン | GitHub Sponsors で応援する | Sponsor on GitHub |
| 補足 | お金を払わなくても、全部の機能を使えます | Every feature is free, with or without sponsoring |

## 6. 変えるファイル

- `src-tauri/Cargo.toml`、`package.json`：上の2つの部品を足す
- `src-tauri/tauri.conf.json`：`bundle.createUpdaterArtifacts: true`、`plugins.updater`（公開鍵・一覧表の場所・Windows は `installMode: passive`）
- `src-tauri/capabilities/*.json`：更新と再起動の許可だけを足す
- `src-tauri/src/lib.rs`：部品の登録。パスワード変更の途中なら更新を断る命令 `can_update`
- `src/lib/updater.ts`（新規）：確かめる、ダウンロード、再起動。デモ版では見本を返す
- `src/components/UpdateBar.tsx`（新規）、`src/views/Overview.tsx`、`src/views/Settings.tsx`、`src/lib/i18n.ts`、`src/lib/demo.ts`、`src/app.css`
- `crates/core` の設定：`auto_update: bool`（最初は true）
- `.github/workflows/build.yml`：署名用の鍵を渡し、.sig と `.app.tar.gz` を作り、`latest.json` を組み立てて置く
- `site/src/i18n.ts`、`site/src/components/Page.astro`：応援の欄、フッター、プライバシー
- `.github/FUNDING.yml`（新規）：`github: kajisho5`

## 7. あなたにお願いすること（1回だけ）

更新ファイルの署名に使う「秘密の鍵」を、GitHub のリポジトリの設定に登録する必要がある。この作業環境からは登録が禁止されている（以前に試して断られた）。
- 鍵は私が作って、コピーボタン付きで渡す
- 登録先：https://github.com/kajisho5/vaultsweep/settings/secrets/actions/new
- 名前 `TAURI_SIGNING_PRIVATE_KEY` で1つ登録する。鍵にはパスワードを付けないので、登録は1つだけで済む
- この鍵を失くすと、配った版に更新を届けられなくなる。1Password に保存しておくこと

鍵が未登録の間も、ビルドとリリースは今までどおり動く。更新ファイルだけ作らずに進めて、作れなかったことを記録に出す。

## 8. サイトごとのパスワードの決まり（デザイン D）

### 分かっていること（事実）
- 1Password CLI で作るときに選べるのは、文字の種類（英字・数字・記号）と長さ（1〜64文字）だけ。記号は「! @ . - _ *」で固定で、記号を1つずつ選ぶことはできない（公式の説明に、選ぶ方法は書かれていない）。根拠：https://www.1password.dev/cli/reference/management-commands/item
- Apple が、サイトごとのパスワードの決まりを約400サイト分公開している（MIT ライセンス）。書き方の例：`minlength: 8; maxlength: 20; required: lower, upper; required: digit; allowed: [!$*?@|]; max-consecutive: 3;`。使われている項目は minlength・maxlength・required・allowed・max-consecutive の5つ。根拠：https://github.com/apple/password-manager-resources （quirks/password-rules.json）
- いまのアプリは、全サイトで同じ決まり（設定の長さ 12〜64文字、記号あり・なし）で作っている

### やること
1. **決まりの取り込み**：Apple の一覧を、ほかの公開一覧と同じく1日に多くても1回取りに行く。取れないときに備えて、アプリにも同じものを入れておく。サイトの判定は今の「事故サイト」と同じく、項目の URL のドメインで行う
2. **作り方を決める**：決まりから、1Password に渡す作り方を決める
   - 長さ：決まりの最大と設定の長さのうち短いほう。最小より短くならないようにする
   - 記号：使える記号が「! @ . - _ *」を全部含むときだけ記号ありにする。含まないなら記号なし
   - 数字だけ（例：6桁の暗証番号）：数字だけで作る
   - 記号が必ず要るのに、1Password の記号の一部が使えないサイト：自動では合わせられない。その旨を出して、自分で決める欄を開く
3. **作ったあとに確かめる**：作ったパスワードを、アプリの中で決まりと照らし合わせる（長さ・使える文字・必ず入れる種類・同じ文字の連続）。合わなければ最大5回まで作り直す
   - 確かめる間も、パスワードは画面に出さず、保存しない。照らし合わせたらすぐにメモリから消す（今の「元に戻す」と同じ扱い）
4. **自分で変える**：「決まりを変える」で、長さと使う文字（英字＋数字＋記号／英字＋数字／数字だけ）を選べる。サイトごとに覚える（パソコンの中の設定ファイルに、ドメインと決まりだけを保存する）
5. **断られたとき**：手順2の画面に「記号なしで作り直す」「16文字で作り直す」「数字だけで作り直す」を置く
   - 作り直しても、預かっている元のパスワードは最初のもののまま。「元に戻す」で、いちばん最初のパスワードに戻る
   - 作り直した決まりは、そのサイトの決まりとして覚える
6. **まとめて片付ける画面**（Batch）でも同じ決まりを使う。断られたときのボタンも同じものを置く

### 変えるファイル（追加分）
- `crates/core/src/rules.rs`（新規）：決まりの読み取り、作り方への変換、照らし合わせ。試験を多めに付ける（Apple の書き方の例、記号の集合の判定、連続の判定）
- `crates/core/src/lists.rs`：一覧の取り込みを足す
- `crates/core/src/op.rs`：作り方に「数字だけ」を足す。長さは 4〜64文字にする（設定の最小 12 はそのまま）
- `crates/core/src/store.rs`：サイトごとの決まり `site_rules`
- `src-tauri/src/lib.rs`：`change_generate` に決まりを渡す。作り直しても元のパスワードを上書きしない
- `src/views/Change.tsx`・`Batch.tsx`・`changeKit.tsx`・`src/components/RuleBox.tsx`（新規）・`src/lib/i18n.ts`・`src/lib/demo.ts`
- 紹介ページのデータ出典に「Apple Password Manager Resources（MIT）」を足す

### 分からないこと（作る前に断っておく）
- 1Password CLI で「数字だけ」（`digits,6` のような書き方）が通るかは、説明に例がなく不明。通らなければ、数字だけのサイトは「自動では合わせられません」の扱いにする
- 1Password が、選んだ種類（大文字・数字など）を必ず1文字以上入れるかは不明。そのために3の照らし合わせを入れる
- 本物の 1Password ではまだ試せない。試験は、作り物の 1Password（試験用の偽物）で行う

## 9. 確かめ方

- 版の比べ方の試験（0.1.0 と 0.2.0、0.1.10 と 0.1.9）
- `latest.json` の形を、公式の説明の項目（version・platforms・url・signature・pub_date）と照らし合わせる
- デモ版で帯とボタンの動き、押せないとき、失敗したとき、英語と日本語の切り替えを、Playwright（ブラウザを自動で操作して画面を確かめる道具）で確かめる
- 紹介ページはパソコン幅とスマホ幅で撮って確かめる
- **確かめられないこと**：実機での入れ替え（0.1.0 から本当に上がるか）。0.1.0 には更新部品が入っていないので、自動更新が効くのは 0.2.0 から 0.2.1 以降になる。0.1.0 を使っている人は、一度だけ手で 0.2.0 を入れ直す必要がある
