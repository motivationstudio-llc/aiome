
/**
 * 認証トークン（シークレット）を取得します。
 * 開発環境では 'dev_secret' をフォールバックとして使用します。
 */
export const getAuthToken = (): string => {
    return localStorage.getItem('aiome_secret') || 'dev_secret';
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
