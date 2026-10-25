# vaultsweep 紹介ページ（GitHub Pages）設計書

状態：**確認待ち**（この設計書と SVG を見てもらってから作る）
デザイン案：`docs/pages-design.svg`（A＝パソコン幅・ダーク、B＝スマホ幅・ライト）

## 1. 何のページか（要件）

- vaultsweep が何をするか分かり、Windows 版・Mac 版をダウンロードできる1枚のページ。
- 日本語と英語。日本語は `/vaultsweep/`、英語は `/vaultsweep/en/`。右上で切り替える。
- ダーク・ライトは、見る人のパソコンやスマホの設定に合わせる（切り替えボタンは付けない）。
- スマホで見ても崩れない（幅 390 で確認）。
- 「1Password 公式のアプリではない」ことを、最初の画面と一番下の両方に書く（なりすましに見えないように）。
- 外から読み込むもの（解析・広告・外部フォント）は入れない。

### 載せる内容（上から順に）

| # | 区切り | 中身 |
|---|---|---|
| 0 | 上の帯 | ロゴ、各区切りへのリンク、EN／日本語、GitHub |
| 1 | 最初の画面 | 見出し・説明・ダウンロード（Windows／Mac）・対応 OS・非公式の注記・アプリの画面写真 |
| 2 | 01 見つけるもの | 8種類の問題（漏洩・事故サイトだけ赤） |
| 3 | 02 片付け方 | 1件ずつ／まとめて／解約 |
| 4 | 03 安全の約束 | パスワードを出さない・送らない・消さない、などの6つ |
| 5 | 04 使い方 | 3つの手順、op の入れ方（Windows／Mac のコマンドとコピーボタン）、Mac で初めて開くときの注意 |
| 6 | 05 AI と使う | MCP の設定例、AI に渡すもの・渡さないもの |
| 7 | 06 よくある質問 | 3つ |
| 8 | 一番下 | ライセンス、ソース、更新履歴、使っているデータの出典、商標の注記 |

## 2. 技術の選び方

| | 動作の軽さ | 配布サイズ | 作りやすさ | 対応 | 将来性 | AI での書きやすさ |
|---|---|---|---|---|---|---|
| **Astro（おすすめ）** | ◎ 動く部分がなく、ほぼ HTML だけ | ◎ 数十 KB＋画像 | ◎ 日本語・英語を同じ部品で作れる | すべてのブラウザ | ◎ 表示中心のサイトの定番 | ◎ |
| 単一 HTML | ◎ | ◎ | △ 2言語ぶんを手で二重に持つ | すべて | ○ | ◎ |
| Vite＋React | ○ 動かすための JS を読み込む | ○ | ○ | すべて | ◎ | ◎ |

**おすすめ：Astro。** 動く部分は「コピー」ボタンだけで、React は要らない。日本語と英語を同じ部品から作れるので、書き換え漏れが起きにくい。

### 使うもの（最新の安定版。2026-10-25 に確認）

| もの | 版 | 根拠 |
|---|---|---|
| Astro | 7.4.2（2026-10-21 公開） | `npm view astro version` ／ https://www.npmjs.com/package/astro |
| フォント Host Grotesk・Atkinson Hyperlegible Mono | 5.3.0（アプリと同じ。ページの中に入れて配る） | https://www.npmjs.com/package/@fontsource-variable/host-grotesk |
| 公開の仕組み | withastro/action@v6、actions/deploy-pages@v5、actions/checkout@v7 | https://docs.astro.build/en/guides/deploy/github/ |

- Tailwind は使わない。アプリと同じ色・余白の決まり（`src/tokens.css`）をそのまま読み込み、見た目をそろえる。

## 3. 見た目（アプリと同じ決まり）

- 色の意味はアプリと同じにする。
  - 琥珀：押すもの（Windows 版のダウンロード）。1画面に1つだけ。
  - 青：見出しの番号・数字・注記。
  - 赤：漏洩・事故サイトだけ。
- 文字は本文が Host Grotesk、数字とコマンドが Atkinson Hyperlegible Mono。日本語は、Windows では Yu Gothic UI、Mac・iPhone ではヒラギノで表示する。
- 箱は最小限にする。区切りは線を基本にし、箱は「片付け方」の3枚、コマンド欄、注意だけ。
- 動きは入れない（見出しの点滅やスクロールで出てくる演出もなし）。

## 4. 部品の分け方

| 部品 | 役目 |
|---|---|
| `Layout.astro` | `<head>`、色の決まり、上の帯、一番下 |
| `Hero.astro` | 見出し・ダウンロード・画面写真 |
| `Kinds.astro` / `Fixes.astro` / `Safety.astro` / `HowTo.astro` / `Mcp.astro` / `Faq.astro` | 各区切り |
| `CopyButton.astro` | コマンドのコピー（ページの中で動く唯一の部品。数行の JS） |
| `i18n.ts` | 日本語・英語の文言（アプリの辞書とは別。ページ用） |

**状態管理：** なし。言語は URL で分け、テーマは OS の設定に任せる。

## 5. データ

- ダウンロードのリンクは GitHub の「最新のリリース」ページへ飛ばす（`/releases/latest`）。
  - ファイル名に版の番号が入るため、ファイルへ直接はリンクしない。
- 版の番号（v0.1.0）は、ページを作るときに `package.json` から読む。
- 画面写真はデモ画面（架空のデータ）から撮る。日本語ページには日本語の写真、英語ページには英語の写真を使う。

## 6. フォルダの構成

```
site/                     ← 紹介ページ（アプリとは別に作る）
  astro.config.mjs        ← site: https://kajisho5.github.io / base: /vaultsweep
  package.json
  src/
    i18n.ts
    layouts/Layout.astro
    components/*.astro
    pages/index.astro     ← 日本語
    pages/en/index.astro  ← 英語
  public/shots/*.png      ← 画面写真（ja / en、ダーク / ライト）
.github/workflows/pages.yml   ← main に push したら公開
```

## 7. 公開の手順（作ったあと）

1. GitHub で Pages を「GitHub Actions から公開」に切り替える。
   - 開く場所：https://github.com/kajisho5/vaultsweep/settings/pages
   - 押すところ：「Build and deployment」→「Source」を「GitHub Actions」にする。
2. main に push すると、`https://kajisho5.github.io/vaultsweep/` に出る。
3. ダウンロードボタンを生かすには、リリースが要る。
   - 「v0.1.0」という印（タグ）を付けると、自動でリリースが作られ、Windows 版と Mac 版が置かれる。
   - タグは公開の操作になるので、付けてよいか確認してから行う。

## 8. まだ決まっていないこと

- Mac 版の署名は、今は仮の署名なので、最初に開くときに止められる（ページに開き方を書く）。
  - Apple の公証を通すには、Apple Developer の証明書を GitHub に登録する必要がある。
