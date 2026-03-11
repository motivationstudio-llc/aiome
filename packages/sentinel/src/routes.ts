import { native } from './native';

export function registerRoutes(api: any) {
    // Expose dashboard or endpoints for local Legacy Gateway
    api.registerRoute({
        method: "GET",
        path: "/aiome/status",
        handler: async (req: any, res: any) => {
            res.json({
                status: "ok",
                message: "Aiome Core active"
            });
        }
    });
}
