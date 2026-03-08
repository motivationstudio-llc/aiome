"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AiomeContextEngine = void 0;
const native_1 = require("./native");
class AiomeContextEngine {
    info = {
        id: 'aiome',
        name: 'Aiome DAE',
        version: '1.0.0',
        ownsCompaction: true,
    };
    async bootstrap({ sessionId }) {
        await native_1.native.karmaBootstrap(sessionId);
    }
    async ingest({ sessionId, message }) {
        if (native_1.native.karmaIngest) {
            await native_1.native.karmaIngest(sessionId, JSON.stringify(message));
        }
    }
    async afterTurn({ sessionId, messages }) {
        if (native_1.native.karmaDistillTurn) {
            await native_1.native.karmaDistillTurn(JSON.stringify(messages), true);
        }
    }
    async assemble({ sessionId, messages, tokenBudget }) {
        // We get warnings and karmas from Rust backend
        const karmas = native_1.native.karmaFetchRelevant ? await native_1.native.karmaFetchRelevant(sessionId, 5) : "";
        const warnings = native_1.native.immuneGetWarnings ? native_1.native.immuneGetWarnings() : "";
        return {
            messages,
            estimatedTokens: messages.length * 50, // naive estimate
            prependSystemContext: warnings || undefined,
            prependContext: karmas || undefined,
        };
    }
    async compact({ sessionId, sessionFile, tokenBudget }) {
        if (native_1.native.karmaCompact) {
            await native_1.native.karmaCompact(sessionId, sessionFile, tokenBudget);
        }
    }
    async prepareSubagentSpawn({ childSessionKey }) {
        if (native_1.native.quarantineCheckSpawn) {
            return await native_1.native.quarantineCheckSpawn(childSessionKey);
        }
        return { status: "ok" };
    }
    async onSubagentEnded({ targetSessionKey, outcome }) {
        if (native_1.native.karmaLearnFromSubagent) {
            await native_1.native.karmaLearnFromSubagent(targetSessionKey, outcome);
        }
    }
    dispose() {
        if (native_1.native.shutdown) {
            native_1.native.shutdown();
        }
    }
}
exports.AiomeContextEngine = AiomeContextEngine;
