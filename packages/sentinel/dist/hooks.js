"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerHooks = registerHooks;
const native_1 = require("./native");
function registerHooks(api) {
    const p0Opts = { priority: -9999 };
    // 1. Sentinel
    api.on("before_tool_call", async (event) => {
        if (native_1.native.immuneCheckTool) {
            const res = await native_1.native.immuneCheckTool(event.toolName, JSON.stringify(event.params));
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
    api.on("after_tool_call", async (event) => {
        if (native_1.native.karmaLearnFromTool) {
            await native_1.native.karmaLearnFromTool(event.toolName, JSON.stringify(event.result), event.error || "");
        }
    }, p0Opts);
    // 3. Karma inject
    api.on("before_prompt_build", async (event, ctx) => {
        if (native_1.native.karmaFetchRelevant) {
            const karma = await native_1.native.karmaFetchRelevant(ctx.sessionId, 5);
            return { prependContext: karma || undefined };
        }
    }, p0Opts);
    // 4. Karma preserve
    api.on("before_compaction", async (event) => {
        if (native_1.native.karmaPreserveFacts) {
            await native_1.native.karmaPreserveFacts(event.sessionFile);
        }
    }, p0Opts);
    // 8. Input monitor
    api.on("llm_input", async (event) => {
        if (native_1.native.immuneScanInput) {
            await native_1.native.immuneScanInput(event.prompt, JSON.stringify(event.historyMessages));
        }
    }, p0Opts);
    // 10. Distill trigger
    api.on("agent_end", async (event) => {
        if (native_1.native.karmaDistillTurn) {
            await native_1.native.karmaDistillTurn(JSON.stringify(event.messages), event.success);
        }
    }, p0Opts);
    // 18. Karma bootstrap
    api.on("session_start", async (event) => {
        if (native_1.native.karmaBootstrap) {
            await native_1.native.karmaBootstrap(event.sessionId);
        }
    }, p0Opts);
    // 19. Karma flush
    api.on("session_end", async (event) => {
        if (native_1.native.karmaFlushSession) {
            await native_1.native.karmaFlushSession(event.sessionId);
        }
    }, p0Opts);
    // Other priority hooks (P1-P3)
    const p1Opts = { priority: -100 };
    api.on("llm_output", async (event) => {
        if (native_1.native.watchtowerTrackUsage && event.usage) {
            await native_1.native.watchtowerTrackUsage(JSON.stringify(event.usage));
        }
    }, p1Opts);
    api.on("gateway_start", async () => {
        if (native_1.native.watchtowerInit)
            await native_1.native.watchtowerInit();
    }, p1Opts);
    api.on("gateway_stop", async () => {
        if (native_1.native.watchtowerShutdown)
            native_1.native.watchtowerShutdown();
    }, p1Opts);
    // Note: Only listing a subset here for now, full 25 hooks will be mapped similarly
    api.logger.info("Aiome Hooks registered (P0 & P1).");
}
