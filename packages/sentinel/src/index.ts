/**
 * @aiome / sentinel Legacy Integration Plugin
 */

import { native } from './native';

import { AiomeContextEngine } from './engine';
import { registerHooks } from './hooks';
import { registerCommands } from './cli/commands';
import { registerRoutes } from './routes';

export function register(api: any) {
    api.logger.info("Initializing @aiome/sentinel plugin...");

    // Register the Context Engine
    api.registerContextEngine("aiome", (engineCtx: any) => new AiomeContextEngine());

    // Register hooks
    registerHooks(api);

    // Register CLI & Routes (Phase P5 & P6)
    if (api.registerCommand) registerCommands(api);
    if (api.registerRoute) registerRoutes(api);
}
