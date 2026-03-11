# CSS Architecture & Layout Patterns

このスキルは、プロジェクトにおける UI レイアウト、特に絶対配置（`position: absolute` / `fixed`）を持つオーバーレイ要素やコンポーネントの位置合わせに関する設計原則（Karma）を規定します。
UIの変更によってレイアウトが崩れる（ズレる）のを防ぐための重要なルールです。

## 1. Single Source of Truth (単一の真実の情報源)

**レイアウトに関する具体的なピクセル数値（幅、高さ、パディング、ギャップなど）を、個別のReactコンポーネントにハードコードしてはいけません。**

すべてのベースとなるレイアウト寸法は、必ず `src/styles/tokens.css` のルート変数（Custom Properties）として一元管理してください。

```css
/* 良い例: tokens.css で一元管理 */
:root {
  --layout-sidebar-width: 280px;
  --layout-main-padding: 3rem;
  --layout-panel-gap: 1.5rem;
}
```

## 2. calc() と var() を用いた自動追従

オーバーレイ要素などをメインコンテンツの中央や特定の位置に合わせる場合、コンポーネント側に直接 `[280px]` などのオフセットを書くのではなく、CSS変数を用いた計算式を実装してください。

### 悪い例（ハードコードによる負債）
```tsx
// ❌ サイドバーや右パネルの幅をハードコードしている
// どれか1つのデザインが変わると、このコンポーネントの位置がズレてしまう。
const leftOffset = "280px";
const rightOffset = "calc(3rem + 320px + 1.5rem)";

return (
  <div style={{ position: 'fixed', left: leftOffset, right: rightOffset }}>
    <Avatar />
  </div>
);
```

### 良い例（変数を参照する自動追従）
```tsx
// ✅ CSSレイアウト変数を参照し、calc でオフセットを動的に決定する
// UIの変更時は tokens.css の変数を書き換えるだけで、このコンポーネントも自動的に正しい位置に追従する。
const leftOffset = "calc(var(--layout-sidebar-width) + var(--layout-main-padding))";
const rightOffset = "calc(var(--layout-main-padding) + var(--layout-right-panel-width) + var(--layout-panel-gap))";

return (
  <div style={{ position: 'fixed', left: leftOffset, right: rightOffset }}>
    <Avatar />
  </div>
);
```

## 3. レスポンシブとの連携

特定のブレイクポイントでサイドバーが消えたり、パネルの幅が変わる場合でも、この設計に従っていれば**メディアクエリ内でCSS変数の値を上書きするだけ**で済みます。

```css
/* モバイル対応もCSS変数の書き換えだけで完結する */
@media (max-width: 768px) {
  :root {
    --layout-sidebar-width: 0px; /* サイドバーを隠す */
  }
}
```

今後は、Reactコンポーネント内でレイアウトのズレを調整する際は、「コンポーネントの数値をいじる」のではなく、「一元管理されたCSS変数を正しく参照できているか」を第一に確認してください。
