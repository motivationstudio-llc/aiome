import { native } from '../native';

export function registerCommands(api: any) {
    // Phase 5: /aiome-status
    api.registerCommand({
        name: "aiome-status",
        description: "Check Aiome Core / Watchtower status",
        handler: async (args: string[]) => {
            // we can retrieve stats via watchtower getAgentStats later
            return {
                text: "Aiome Core is running. Watchtower and Immune Systems active."
            };
        }
    });
}
