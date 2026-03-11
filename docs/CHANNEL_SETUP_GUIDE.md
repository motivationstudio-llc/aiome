# 📡 Channel Integration Setup Guide

Aiome を Discord や Telegram と接続し、チャットプラットフォーム上で AI と対話できるようにするための完全なセットアップガイドです。

> [!NOTE]
> このガイドは初めて Bot を作成する方を対象としています。所要時間は各プラットフォームにつき約 5〜10 分です。

---

## 目次

1. [Telegram Bot のセットアップ](#1-telegram-bot-のセットアップ)
2. [Discord Bot のセットアップ](#2-discord-bot-のセットアップ)
3. [Aiome ダッシュボードへの設定入力](#3-aiome-ダッシュボードへの設定入力)
4. [接続テスト](#4-接続テスト)
5. [トラブルシューティング](#5-トラブルシューティング)

---

## 1. Telegram Bot のセットアップ

### 1-1. Bot の作成と Token の取得

Telegram の Bot 管理はすべて **BotFather** という公式 Bot とのチャットで行います。

1. Telegram アプリを開き、検索バーで **`@BotFather`** を検索します。
   - 青い公式チェックマーク ✅ がついているものを選択してください。
2. BotFather とのチャットを開き、画面下の **「Start」** をタップします。
3. チャット欄に **`/newbot`** と入力して送信します。
4. **Bot の表示名** を聞かれるので、任意の名前を入力します。（例：`Aiome Assistant`）
5. **Bot のユーザー名** を聞かれます。末尾は必ず `bot` または `_bot` で終わる必要があります。（例：`my_aiome_bot`）
6. 作成に成功すると、以下の形式の文字列が表示されます。**これが Telegram Token です。**

```
Use this token to access the HTTP API:
1234567890:ABCdefGHIjklMNOpqrSTUvwxyz12345678
```

> [!CAUTION]
> Token はパスワードと同等の機密情報です。Git にコミットしたり、他人に共有しないでください。`.env` ファイルに `TELEGRAM_TOKEN=取得したToken` として安全に保存してください。

### 1-2. Chat ID の取得

Bot がメッセージを送信する宛先を特定するために、Chat ID が必要です。

#### 個人チャット（自分だけの通知用）の場合

1. Telegram の検索バーで **`@userinfobot`** を検索します。
2. そのボットとのチャットを開き、**「Start」** をタップします。
3. ボットが返信に **`Id: 123456789`** のような数字を表示します。
4. **この数字があなたの Chat ID です。**

#### グループチャットの場合

1. 作成した Bot を対象の Telegram グループに招待（Add to Group）します。
2. グループ内で適当なメッセージ（例：`テスト`）を送信します。
3. ブラウザで以下の URL にアクセスします（`<TOKEN>` は取得した Telegram Token に置き換え）。

```
https://api.telegram.org/bot<TOKEN>/getUpdates
```

4. 表示される JSON の中から `"chat":{"id": -1001234567890, ...}` を探します。
5. **`-100...` から始まる数字（マイナス記号を含む）がグループの Chat ID です。**

> [!TIP]
> ブラウザに `{"ok":true,"result":[]}` しか表示されない場合は、グループ内でメッセージを送信してからブラウザを更新してください。

### 1-3. Bot への対話許可（必須）

Telegram のスパム防止ルールにより、**ユーザー側から先に Bot に対して対話を開始する必要があります。**

1. Telegram の検索バーで、作成した Bot のユーザー名（`@xxxxx_bot`）を検索します。
2. Bot とのチャットを開き、**「Start」** をタップ（または `/start` と送信）します。

> [!IMPORTANT]
> この操作を行わないと、Bot からメッセージを送信しても `chat not found` エラーが発生します。

---

## 2. Discord Bot のセットアップ

### 2-1. アプリケーションの作成と Token の取得

1. ブラウザで [Discord Developer Portal](https://discord.com/developers/applications) にアクセスし、Discord アカウントでログインします。
2. 右上の **[New Application]** をクリックします。
3. アプリ名（例：`Aiome Bot`）を入力し、規約に同意して **[Create]** を押します。
4. 左側メニューの **[Bot]** を選択します。
5. **[Reset Token]** をクリックし、表示された Token をコピーします。

> [!CAUTION]
> Token は**この画面を閉じると二度と表示されません。** 必ずすぐに `.env` ファイルの `DISCORD_TOKEN` に保存してください。

### 2-2. Gateway Intents（特権インテンツ）の有効化

同じ **[Bot]** メニュー画面を下へスクロールし、「Privileged Gateway Intents」の以下 3 つのスイッチをすべて **ON（青色）** にして **[Save Changes]** を押します。

| Intent | 用途 |
|--------|------|
| `PRESENCE INTENT` | ユーザーのオンライン状態を検知 |
| `SERVER MEMBERS INTENT` | メンバー一覧の取得 |
| `MESSAGE CONTENT INTENT` | メッセージ内容の読み取り（**必須**） |

> [!WARNING]
> `MESSAGE CONTENT INTENT` が OFF のままだと、Bot はメッセージの内容を一切読み取れません。必ず ON にしてください。

### 2-3. Bot のサーバーへの招待

1. 左側メニューから **[OAuth2]** → **[URL Generator]** を選択します。
2. 「SCOPES」の中で **`bot`** にチェックを入れます。
3. 新たに表示される「BOT PERMISSIONS」で以下にチェックを入れます。

| 権限 | 説明 |
|------|------|
| `Read Messages/View Channels` | チャンネルとメッセージの閲覧 |
| `Send Messages` | メッセージの送信 |
| `Read Message History` | 過去メッセージの閲覧 |
| `Embed Links` | リッチ埋め込みの送信（推奨） |

4. 画面最下部の **「GENERATED URL」** をコピーします。
5. ブラウザの新しいタブに貼り付けてアクセスします。
6. Bot を追加するサーバーを選択し、**「認証」** を押して招待を完了します。

### 2-4. チャンネル ID の取得

1. Discord アプリ（またはWeb版）を開きます。
2. **ユーザー設定** → **詳細設定** → **開発者モード** を **ON** にします。
3. Aiome に発言させたいチャンネル（例：`#general`）を**右クリック**します。
4. **「チャンネル ID をコピー」** を選択します。（18 桁前後の数字）

---

## 3. Aiome ダッシュボードへの設定入力

### 3-1. Token の設定（`.env` ファイル）

プロジェクトルートの `.env` ファイルに以下を記述します。

```env
# Telegram
TELEGRAM_TOKEN=取得したTelegramToken

# Discord
DISCORD_TOKEN=取得したDiscordToken
```

> [!IMPORTANT]
> `.env` ファイルの変更を反映するには、`api-server` の再起動が必要です。

### 3-2. チャンネル ID の設定（ダッシュボード）

1. 管理画面（Management Console）を開きます。
2. サイドバーの **[Settings]** をクリックします。
3. **「Channel Bridges」** セクションで以下を入力します。

| 項目 | 入力する値 |
|------|-----------|
| Discord Chat Channel ID | 手順 2-4 で取得した 18 桁の数字 |
| Telegram Chat ID | 手順 1-2 で取得した Chat ID |

4. **「Enable Watchtower」** トグルを **ON** にします。

---

## 4. 接続テスト

すべての設定が完了したら、Management Console の **Agent Console** タブからメッセージを送信してください。正しく設定されていれば、Discord と Telegram の両方に Aiome の応答が配信されます。

また、以下の `curl` コマンドでプラットフォーム単体のテストも可能です。

```bash
# Telegram テスト
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"chat_id": "<CHAT_ID>", "text": "Aiome Test 🚀"}' \
  https://api.telegram.org/bot<TOKEN>/sendMessage

# Discord テスト
curl -X POST \
  -H "Authorization: Bot <TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"content": "Aiome Test 🚀"}' \
  https://discord.com/api/v10/channels/<CHANNEL_ID>/messages
```

---

## 5. トラブルシューティング

### Telegram

| エラー | 原因と対策 |
|--------|-----------|
| `chat not found` | Bot に対して `/start` を送信していない。Bot のチャットを開いて「Start」をタップしてください。 |
| `Unauthorized` | Token が無効です。BotFather で `/token` を送信して最新の Token を再取得してください。 |
| `Bad Request: group chat was deactivated` | グループが削除されたか、Bot がキックされています。再度招待してください。 |

### Discord

| エラー | 原因と対策 |
|--------|-----------|
| `Missing Access` | Bot がサーバーに招待されていないか、チャンネルへの権限がありません。手順 2-3 を再実行してください。 |
| `Unknown Channel` | チャンネル ID が間違っています。開発者モードで再取得してください。 |
| `Unauthorized` | Token が無効です。Developer Portal で Token を再生成してください。 |
| メッセージが読めない | `MESSAGE CONTENT INTENT` が OFF です。Developer Portal の Bot 設定で ON にしてください。 |

---

> [!NOTE]
> セキュリティ上の理由から、Discord Token と Telegram Token は `.env` ファイル（環境変数）で管理され、ダッシュボード上では「Vault Protected」として保護されています。データベースには保存されません。
