import { native } from './native';
export function registerHooks(api: any) {
    const p0Opts = { priority: -9999 };

    // 1. Sentinel
    api.on("before_tool_call", async (event: any) => {
        if (native.immuneCheckTool) {
            const res = await native.immuneCheckTool(event.toolName, JSON.stringify(event.params));
            if (res.blocked) {
                return { block: true, blockReason: res.reason };
            }
            if (res.newParams) {
                return { params: JSON.parse(res.newParams) };
            }
        }
        return {};
    }, p0Opts);

    // 2. Karma learning
    api.on("after_tool_call", async (event: any) => {
        if (native.karmaLearnFromTool) {
            await native.karmaLearnFromTool(event.toolName, JSON.stringify(event.result), event.error || "");
        }
    }, p0Opts);

    // 3. Karma inject
    api.on("before_prompt_build", async (event: any, ctx: any) => {
        if (native.karmaFetchRelevant) {
            const karma = await native.karmaFetchRelevant(ctx.sessionId, 5);
            return { prependContext: karma || undefined };
        }
    }, p0Opts);

    // 4. Karma preserve
    api.on("before_compaction", async (event: any) => {
        if (native.karmaPreserveFacts) {
            await native.karmaPreserveFacts(event.sessionFile);
        }
    }, p0Opts);

    // 8. Input monitor
    api.on("llm_input", async (event: any) => {
        if (native.immuneScanInput) {
            await native.immuneScanInput(event.prompt, JSON.stringify(event.historyMessages));
        }
    }, p0Opts);

    // 10. Distill trigger
    api.on("agent_end", async (event: any) => {
        if (native.karmaDistillTurn) {
            await native.karmaDistillTurn(JSON.stringify(event.messages), event.success);
        }
    }, p0Opts);

    // 18. Karma bootstrap
    api.on("session_start", async (event: any) => {
        if (native.karmaBootstrap) {
            await native.karmaBootstrap(event.sessionId);
        }
    }, p0Opts);

    // 19. Karma flush
    api.on("session_end", async (event: any) => {
        if (native.karmaFlushSession) {
            await native.karmaFlushSession(event.sessionId);
        }
    }, p0Opts);

    // Other priority hooks (P1-P3)
    const p1Opts = { priority: -100 };

    api.on("llm_output", async (event: any) => {
        if (native.watchtowerTrackUsage && event.usage) {
            await native.watchtowerTrackUsage(JSON.stringify(event.usage));
        }
    }, p1Opts);

    api.on("gateway_start", async () => {
        if (native.watchtowerInit) await native.watchtowerInit();
    }, p1Opts);

    api.on("gateway_stop", async () => {
        if (native.watchtowerShutdown) native.watchtowerShutdown();
    }, p1Opts);

    // Note: Only listing a subset here for now, full 25 hooks will be mapped similarly
    api.logger.info("Aiome Hooks registered (P0 & P1).");
}
