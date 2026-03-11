# Aiome Console - レイアウトアーキテクチャ仕様

## 概要

DioramaView（アバターオーバーレイ）などの絶対配置要素が、UIのコンテンツ領域の変更に影響されずに
常に正しい位置に配置されるようにするため、**CSS Custom Properties（CSS変数）によるレイアウト寸法の一元管理**を導入しています。

## Single Source of Truth: `tokens.css`

すべてのレイアウト寸法は `src/styles/tokens.css` の `:root` に定義されています。

```css
/* ========================================
 * Layout Dimensions (Single Source of Truth)
 * ========================================
 * DioramaView (avatar overlay) はこれらの変数を参照して
 * メインコンテンツ領域に対して正確に中央揃えされる。
 * レイアウトを変更する場合はここだけ修正すればよい。
 */
--layout-sidebar-width: 280px;
--layout-main-padding: var(--space-xl);          /* 3rem */
--layout-panel-gap: var(--space-md);              /* 1.5rem */
--layout-right-panel-width: 320px;
```

## 参照マップ

| CSS 変数 | 参照元 | 用途 |
|---|---|---|
| `--layout-sidebar-width` | `App.css` (`.sidebar`) | サイドバーの幅幅 |
| `--layout-main-padding` | `App.css` (`.main-content`) | メインコンテンツの外側余白 |
| `--layout-right-panel-width` | `BiotopeView.tsx` (grid) | Biotope画面の右パネルの幅 |
| `--layout-panel-gap` | `BiotopeView.tsx` (grid) | Biotope画面グリッドのギャップ |
| **上記すべて** | `DioramaView.tsx` | アバターオーバーレイのオフセット（inset）計算 |

## DioramaView のオフセット計算（自動追従）

`DioramaView` の位置合わせは、画面サイズを固定で引くのではなく、
「サイドバー」や「右パネル」などの具体的な変数を用いた CSS の `calc()` を使ってダイナミックに算出されます。

```
┌─────────────────────────────────────────────────────────────┐
│ Window                                                       │
│ ┌──────────┐ ┌───────────────────────────────┐ ┌──────────┐ │
│ │          │P│                               │G│          │P│
│ │ Sidebar  │A│   Avatar Center Area          │A│  Right   │A│
│ │          │D│   (DioramaView covers here)   │P│  Panel   │D│
│ │          │ │                               │ │          │ │
│ └──────────┘ └───────────────────────────────┘ └──────────┘ │
│              ↑                                 ↑             │
│          leftOffset                       rightOffset        │
└─────────────────────────────────────────────────────────────┘

leftOffset  = calc(var(--layout-sidebar-width) + var(--layout-main-padding))
rightOffset = calc(var(--layout-main-padding) + var(--layout-right-panel-width) + var(--layout-panel-gap))
              ※ ダッシュボード画面以外では右パネルがないため var(--layout-main-padding) のみ
```

## 変更ガイド

> [!IMPORTANT]
> このレイアウト仕様があるため、レイアウト寸法を変更する場合は各Reactコンポーネントを直接編集する必要はありません。
> **`tokens.css` の変数を変更するだけで、UI全体のグリッドとアバターの位置がピクセル単位で自動的に追従します。**

### 例: サイドバーの幅を 280px から 300px に変える場合
`tokens.css` の `--layout-sidebar-width: 300px;` と変更するだけで、自動的にアバターはそれに合わせて20px分右寄りに補正され、背景のサークルと完璧に重なり続けます。
