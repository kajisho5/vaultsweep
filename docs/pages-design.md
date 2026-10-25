# vaultsweep 紹介ページ（GitHub Pages）設計書 — 案3（英語が主・日本語はサブ）

状態：**確認待ち**（この設計書と SVG を見てもらってから作る）
デザイン案：`docs/pages-design.svg`（A＝英語ページ・パソコン幅・ダーク、B＝日本語ページ・スマホ幅・ライト／Mac で開いた例）

**言語：英語が主。** `https://kajisho5.github.io/vaultsweep/` が英語、`/vaultsweep/ja/` が日本語。上の帯と一番下に切り替えリンクを置く。見ている人の言語で勝手に飛ばすことはしない（検索エンジンが英語ページを正しく読めるように）。`hreflang` で英語・日本語の対応を知らせる。
使うスキル：`landing-page-design`（elayadesign/ai-design-skills、2026-10-25 時点の版 38bbe57）

## 0. 前提（スキルの「最初に聞くこと」への答え。分からない所は仮に決めた）

| 項目 | 答え |
|---|---|
| 一番してほしいこと | アプリをダウンロードしてもらう |
| 何がもらえるか | 無料のアプリ（Windows・Mac） |
| 誰向けか | 世界中の 1Password 利用者（英語が主）。漏洩や使い回しが気になっているのに、手をつけていない人 |
| ためらう理由（仮） | ①パスワードを他のアプリに触らせるのが怖い ②壊したら困る ③設定が面倒そう |
| 来る道（仮） | GitHub・SNS・検索 |
| 証拠に使えるもの | オープンソース、安全の約束、実際の画面。お客様の声や数字は**載せない**（作り話になるため） |
| 雰囲気 | 落ち着いた、無駄のない見た目。スマホでも読める |

## 1. ページの流れ（スキルの A2 に沿う）／型は「A. 最初の画面＋区切り」

選んだ理由：アプリの画面写真を見れば何をするものか分かるので、最初の画面に写真を大きく置く型が合う。

| # | 区切り | 中身 | 3つのためらいへの答え |
|---|---|---|---|
| 1 | 上の帯 | 浮いた丸いナビ（ロゴ・4つのリンク・EN・GitHub） | |
| 2 | 最初の画面 | 見出し・説明・**主ボタン1つ**・Mac 版の文字リンク・証拠の一行・画面写真 | ① |
| 3 | よいこと | 4枚のカード | ② |
| 4 | 大きな一言 | 「パスワードは、あなたのパソコンから出ていかない。片付けるのは、あなたが押したときだけ。」スクロールに合わせて1語ずつ色が付く | ① |
| 5 | 見つけるもの | 8種類（漏洩・事故サイトだけ赤） | |
| 6 | 3つの手順 | カード3枚＋コマンド（コピーボタン付き）＋ Mac で初めて開くときの注意 | ③ |
| 7 | 確かめられる安全 | 6つの約束＋ソースコードへのリンク（お客様の声の代わりの証拠） | ①② |
| 8 | AI と使う | MCP の設定例、AI に渡すもの・渡さないもの | ① |
| 9 | よくある質問 | 8問 | ①②③ |
| 10 | 最後の誘い | 戻せること（安心材料）＋最初の画面と同じボタン | ② |
| 11 | 一番下 | プライバシー・ライセンス・ソース・更新履歴・English、データの出典、商標の注記 | |

### 最初の画面の言葉（英語／日本語）

| | 英語（主） | 日本語 |
|---|---|---|
| 見出し | Clean up leaked and／reused passwords／in 1Password tonight. | 漏れた・使い回した／パスワードを、／今夜のうちに片付ける。 |
| 説明 | vaultsweep scans your 1Password vault, ranks what is risky, and walks you through changing passwords and closing accounts one at a time. Free for Windows and Mac. | vaultsweep は 1Password を診断して、危ない順に並べ、変更と解約を1件ずつ手伝う無料のアプリです。Windows と Mac で動きます。 |
| 主ボタン | Download free for Windows（Mac で見ていれば for Mac） | Windows 版を無料でダウンロード（Mac なら Mac 版） |
| 証拠の一行 | Open source (MIT) · Your passwords never leave your computer · v0.1.0 | オープンソース（MIT）· パスワードはパソコンの外に出ません · v0.1.0 |
| 大きな一言 | Your passwords never leave your computer. Nothing changes until you press the button. | パスワードは、あなたのパソコンから出ていかない。片付けるのは、あなたが押したときだけ。 |

- 主ボタンでは、見ている人のパソコンが Windows か Mac かを判定し、合う方を出す。判定できなければ Windows を出す。もう一方は文字のリンクにする。

### よいこと（4つ）

1. 危ない順に、数分で分かる
2. 1件ずつ、迷わず変えられる（だめなら元に戻せる）
3. まとめて、続けて片付く（Enter で次へ、S で「あとで」）
4. 使わないサイトは、解約まで（ダミー情報 → 退会 → 記録 → 保管庫）

### よくある質問（8問）

お金・外に送られるか・公式か・間違えて消したら・変えられなかったら・パスキー・Mac で開けない・自動診断。

### 検索での見え方（SEO・AEO）

- 検索に**出す**（ずっと使える内容のため）。
- 英語の題名：「vaultsweep: find and fix leaked and reused passwords in 1Password」
- 英語の説明文：「Free, open source app for Windows and Mac. Scans your 1Password vault, ranks risky logins, and helps you change passwords and close accounts.」
- 日本語ページにも、同じ内容の日本語の題名と説明文を付ける。
- よくある質問は「質問と答え」の形のデータ（FAQ スキーマ）も付ける。

## 2. 見た目の決まり（スキルの B に沿う）

| 項目 | 決まり |
|---|---|
| 字 | Geist の1種類（コマンドと数字だけ Geist Mono）。日本語は Mac・iPhone ではヒラギノ、Windows では游ゴシックで表示。斜体・極太は使わない |
| 字の大きさ | Tailwind の段だけ。見出し 60px（スマホ 36px）、区切りの見出し 36px、本文 18／16px、ボタン 16px（太字） |
| 余白 | 2・4・8・12・16・24・32・40・48・64・80・96px だけ。区切りの間は 96px |
| 角の丸み | カード 16px、ボタン 8px、ナビは丸い錠剤形。入れ子の丸みは「外側の丸み−すき間」 |
| 背景 | 平らな色だけ。ダークは #000000、カードは #181818、線は #313131。ライトは #FAFAFA と白 |
| 最初の見出し | 左から右へ、白→#9B9B9B の字のグラデーション（ライトでは黒→#666666）。横幅は 680px まで、意味の切れ目で改行 |
| 色の意味 | 琥珀＝主ボタン（1画面に1つ）、赤＝漏洩・事故サイトだけ。アプリと同じ |
| アイコン | Phosphor |
| 動き | 決めたなめらかな曲線で 0.7〜0.8 秒。カードと見出しは画面に入るときに下から浮かび上がる。大きな一言は1語ずつ色が付く。ナビの「≡」は「×」に形を変えて開く。**動きを減らす設定の人には動かさない**（スキルにはない、こちらで足した決まり） |
| 状態 | すべてのボタンとリンクに、指を置いたとき・押したとき・キーボードで選んだときの見た目を付ける |
| 文の決まり | 文中にハイフンを使わない（SHA-1 は「指紋（ハッシュ）」と書く）。最後の行に1語だけが残らないよう、日本語は文節で改行する |

## 3. 技術の選び方

| | 動作の軽さ | 配布サイズ | 作りやすさ | 対応 | 将来性 | AI での書きやすさ |
|---|---|---|---|---|---|---|
| **Astro＋Tailwind（おすすめ）** | ◎ ほぼ HTML。動きは小さな JS だけ | ◎ | ◎ スキルの決まりが Tailwind の段で書いてある。日本語・英語を同じ部品で作れる | すべて | ◎ | ◎ |
| 単一 HTML | ◎ | ◎ | △ 2言語ぶんを手で二重に持つ | すべて | ○ | ◎ |
| Vite＋React | ○ JS が増える | ○ | ○ | すべて | ◎ | ◎ |

**おすすめ：Astro＋Tailwind。** 表示中心のページなので、Astro が一番軽い。スキルの決まりも Tailwind の段のまま書ける。

### 使うもの（最新の安定版。2026-10-25 に確認）

| もの | 版 | 根拠 |
|---|---|---|
| Astro | 7.4.2（2026-10-21） | https://www.npmjs.com/package/astro |
| Tailwind CSS ＋ @tailwindcss/vite | 4.3.3（2026-07-16） | https://www.npmjs.com/package/tailwindcss |
| Geist・Geist Mono（ページの中に入れて配る） | @fontsource-variable/geist 5.3.0（OFL） | https://www.npmjs.com/package/@fontsource-variable/geist |
| Phosphor アイコン（使う分だけ SVG で埋め込む） | @phosphor-icons/core 2.1.1（MIT） | https://www.npmjs.com/package/@phosphor-icons/core |
| 公開 | withastro/action@v6、actions/deploy-pages@v5、actions/checkout@v7 | https://docs.astro.build/en/guides/deploy/github/ |

Phosphor の最新版は 2024-03 公開。古いが、更新が止まっているだけで、非推奨にはなっていない。

## 4. 部品・状態・データ

- 部品：`Layout`、`Nav`（≡→× の切り替え）、`Hero`、`Benefits`、`Tagline`（1語ずつ）、`Kinds`、`HowTo`、`CopyButton`、`Safety`、`Mcp`、`Faq`、`FinalCta`、`Footer`
- 状態：ページの中で持つのは「メニューが開いているか」と「コピーした直後か」だけ。言語は URL で分ける（英語 `/vaultsweep/`、日本語 `/vaultsweep/ja/`）。テーマは OS の設定に任せる。
- 動く部分は小さな素の JS で書く：Mac かどうかの判定、メニューの開閉、コピー、画面に入ったら浮かび上がる、1語ずつ色を付ける。スクロールは `IntersectionObserver` で見る。
- データ：
  - ダウンロードは「最新のリリース」ページへ飛ばす。
  - 版の番号はアプリの `package.json` から読む。
  - 画面写真はデモ（架空のデータ）から撮る。ja と en、ダークとライトを用意する。
- 公開に要るもの：プライバシーのページ（「何も集めない・クッキーなし」）、自前の 404、「本文へ飛ぶ」リンク、ファビコン、`og:image`、すべての画像の代わりの文。

## 5. フォルダの構成

```
site/
  astro.config.mjs   ← site: https://kajisho5.github.io / base: /vaultsweep
  package.json
  src/
    i18n.ts
    layouts/Layout.astro
    components/*.astro
    scripts/motion.ts      ← 浮かび上がり・1語ずつ・メニュー・コピー・Mac 判定
    pages/index.astro（英語） / ja/index.astro / privacy.astro / ja/privacy.astro / 404.astro
  public/shots/*.png  public/og.png  public/favicon.svg
.github/workflows/pages.yml
```

## 6. 公開の手順（作ったあと）

1. GitHub で、Pages を「GitHub Actions から公開」に切り替える。
   - 開く場所：https://github.com/kajisho5/vaultsweep/settings/pages
   - 押すところ：「Build and deployment」→「Source」を「GitHub Actions」にする。
2. main に push すると、英語ページが `https://kajisho5.github.io/vaultsweep/`、日本語ページが `/vaultsweep/ja/` に公開される。
3. ダウンロードボタンを生かすには、リリースが要る。
   - 「v0.1.0」という印（タグ）を付けると、リリースが自動で作られ、Windows 版と Mac 版が置かれる。
   - タグを付けるのは公開の操作なので、付けてよいか確認してから行う。

## 7. スキルの決まりと違うところ（理由つき）

- **お客様の声・数字の実績は載せない**：まだ利用者がいないので、載せれば作り話になる。代わりに、オープンソースであることと安全の約束を証拠にする。
- **動きを減らす設定の人には動かさない**：見やすさのために、こちらで足した決まり。
- **アプリ本体の見た目（Hallmark）とは字や色が違う**：このスキルに沿うと、字は Geist、ダークの背景は真っ黒になる。色の意味（琥珀・赤）だけはアプリとそろえる。
