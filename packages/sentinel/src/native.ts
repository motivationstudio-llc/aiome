import * as os from 'os';

export interface SubagentSpawnResponse {
    status: string;
}

export interface ToolCheckResponse {
    blocked: boolean;
    reason?: string;
    newParams?: string;
}

export interface AiomeNativeBridge {
    karmaBootstrap(sessionId: string): Promise<void>;
    karmaIngest(sessionId: string, message: string): Promise<void>;
    karmaDistillTurn(messages: string, success: boolean): Promise<void>;
    karmaFetchRelevant(sessionId: string, limit: number): Promise<string>;
    get_karma_directives(topic: string, skillId: string): Promise<string>;
    immuneGetWarnings(): Promise<string>;
    karmaCompact(sessionId: string, sessionFile: string, tokenBudget: number): Promise<void>;
    quarantineCheckSpawn(childSessionKey: string): Promise<SubagentSpawnResponse>;
    karmaLearnFromSubagent(targetSessionKey: string, outcome: string): Promise<void>;
    shutdown(): void;
    immuneCheckTool(toolName: string, params: string): Promise<ToolCheckResponse>;
    karmaLearnFromTool(toolName: string, result: string, errorMsg: string): Promise<void>;
    karmaPreserveFacts(sessionFile: string): Promise<void>;
    immuneScanInput(prompt: string, historyMessages: string): Promise<void>;
    karmaFlushSession(sessionId: string): Promise<void>;
    watchtowerTrackUsage(usage: string): Promise<void>;
    watchtowerInit(): Promise<void>;
    watchtowerShutdown(): void;
}

let native: AiomeNativeBridge;

try {
    const platform = os.platform();
    const arch = os.arch();
    native = require(`../../index.${platform}-${arch}.node`) as AiomeNativeBridge;
} catch (e) {
    native = {
        async karmaBootstrap() { },
        async karmaIngest() { },
        async karmaDistillTurn() { },
        async karmaFetchRelevant() { return ""; },
        async get_karma_directives() { return ""; },
        async immuneGetWarnings() { return ""; },
        async karmaCompact() { },
        async quarantineCheckSpawn() { return { status: 'ok' }; },
        async karmaLearnFromSubagent() { },
        shutdown() { },
        async immuneCheckTool() { return { blocked: false }; },
        async karmaLearnFromTool() { },
        async karmaPreserveFacts() { },
        async immuneScanInput() { },
        async karmaFlushSession() { },
        async watchtowerTrackUsage() { },
        async watchtowerInit() { },
        watchtowerShutdown() { }
    } as AiomeNativeBridge;
}

export { native };
