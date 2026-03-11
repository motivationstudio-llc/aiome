
/**
 * 認証トークン（シークレット）を取得します。
 * 開発環境では 'dev_secret' をフォールバックとして使用します。
 */
export const getAuthToken = (): string => {
    const token = localStorage.getItem('aiome_secret') || 'dev_secret';
    const updatedAt = localStorage.getItem('aiome_secret_updated_at');

    if (updatedAt) {
        const age = Date.now() - parseInt(updatedAt);
        if (age > 24 * 60 * 60 * 1000) {
            console.warn("🔐 [Auth] Token is over 24h old. Consider rotation.");
        }
    }

    return token;
};

/**
 * 認証済みの fetch リクエストを実行するためのヘルパー。
 * 将来的に Cookie 認証に移行する場合、この関数内で処理を集約できます。
 */
export const authenticatedFetch = async (url: string, options: RequestInit = {}): Promise<Response> => {
    const token = getAuthToken();
    const headers = {
        ...options.headers,
        'Authorization': `Bearer ${token}`,
    };

    return fetch(url, { ...options, headers });
};

/**
 * API の全エンドポイントで共通して使用する認証ヘッダーを生成します。
 */
export const getAuthHeaders = () => {
    return {
        'Authorization': `Bearer ${getAuthToken()}`,
    };
};
