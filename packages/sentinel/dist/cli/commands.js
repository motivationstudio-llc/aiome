"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerCommands = registerCommands;
function registerCommands(api) {
    // Phase 5: /aiome-status
    api.registerCommand({
        name: "aiome-status",
        description: "Check Aiome Core / Watchtower status",
        handler: async (args) => {
            // we can retrieve stats via watchtower getAgentStats later
            return {
                text: "Aiome Core is running. Watchtower and Immune Systems active."
            };
        }
    });
}
