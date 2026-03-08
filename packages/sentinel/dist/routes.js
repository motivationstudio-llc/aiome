"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerRoutes = registerRoutes;
function registerRoutes(api) {
    // Expose dashboard or endpoints for local OpenClaw Gateway
    api.registerRoute({
        method: "GET",
        path: "/aiome/status",
        handler: async (req, res) => {
            res.json({
                status: "ok",
                message: "Aiome Core active"
            });
        }
    });
}
