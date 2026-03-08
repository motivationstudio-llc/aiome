"use strict";
/**
 * @aiome/sentinel OpenClaw Plugin
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.register = register;
const engine_1 = require("./engine");
const hooks_1 = require("./hooks");
const commands_1 = require("./cli/commands");
const routes_1 = require("./routes");
function register(api) {
    api.logger.info("Initializing @aiome/sentinel plugin...");
    // Register the Context Engine
    api.registerContextEngine("aiome", (engineCtx) => new engine_1.AiomeContextEngine());
    // Register hooks
    (0, hooks_1.registerHooks)(api);
    // Register CLI & Routes (Phase P5 & P6)
    if (api.registerCommand)
        (0, commands_1.registerCommands)(api);
    if (api.registerRoute)
        (0, routes_1.registerRoutes)(api);
}
