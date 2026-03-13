
/**
 * セキュア認証トークン管理
 * - sessionStorage を使用（ブラウザ閉鎖で自動消去）
 * - 本番環境での 'dev_secret' フォールバックを廃止
 */
export const getAuthToken = (): string | null => {
    return sessionStorage.getItem('aiome_secret');
};

/**
 * 認証済みの fetch リクエストを実行するためのヘルパー。
 */
export const authenticatedFetch = async (url: string, options: RequestInit = {}): Promise<Response> => {
    const token = getAuthToken();
    const headers = {
        ...options.headers,
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    };

    return fetch(url, { ...options, headers });
};

/**
 * API の全エンドポイントで共通して使用する認証ヘッダーを生成します。
 */
export const getAuthHeaders = () => {
    const token = getAuthToken();
    return {
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    };
};

/**
 * トークンを SessionStorage に保存します。
 */
export const setAuthToken = (token: string): void => {
    sessionStorage.setItem('aiome_secret', token);
    sessionStorage.setItem('aiome_secret_updated_at', Date.now().toString());
};

/**
 * トークンを SessionStorage から削除します。
 */
export const clearAuthToken = (): void => {
    sessionStorage.removeItem('aiome_secret');
    sessionStorage.removeItem('aiome_secret_updated_at');
};

/**
 * 現在認証されているかどうかを判定します。
 */
export const isAuthenticated = (): boolean => {
    return getAuthToken() !== null;
};
