import { native } from './native';

export class AiomeContextEngine {
    readonly info = {
        id: 'aiome',
        name: 'Aiome DAE',
        version: '1.0.0',
        ownsCompaction: true,
    };

    async bootstrap({ sessionId }: any) {
        await native.karmaBootstrap(sessionId);
    }

    async ingest({ sessionId, message }: any) {
        if (native.karmaIngest) {
            await native.karmaIngest(sessionId, JSON.stringify(message));
        }
    }

    async afterTurn({ sessionId, messages }: any) {
        if (native.karmaDistillTurn) {
            await native.karmaDistillTurn(JSON.stringify(messages), true);
        }
    }

    async assemble({ sessionId, messages, tokenBudget }: any) {
        // We get warnings and karmas from Rust backend
        const karmas = native.karmaFetchRelevant ? await native.karmaFetchRelevant(sessionId, 5) : "";
        const warnings = native.immuneGetWarnings ? await native.immuneGetWarnings() : "";

        // Also fetch directives if available
        const topic = messages[messages.length - 1]?.content || "";
        const directives = native.get_karma_directives ? await native.get_karma_directives(topic, "global") : "";

        const estimatedTokens = messages.reduce((acc: number, m: any) => {
            const contentLen = typeof m.content === 'string' ? m.content.length : JSON.stringify(m.content || '').length;
            return acc + Math.ceil(contentLen / 3);
        }, 0);

        return {
            messages,
            estimatedTokens,
            prependSystemContext: (warnings || "") + (directives || "") || undefined,
            prependContext: karmas || undefined,
        };
    }

    async compact({ sessionId, sessionFile, tokenBudget }: any) {
        if (native.karmaCompact) {
            await native.karmaCompact(sessionId, sessionFile, tokenBudget);
        }
    }

    async prepareSubagentSpawn({ childSessionKey }: any) {
        if (native.quarantineCheckSpawn) {
            return await native.quarantineCheckSpawn(childSessionKey);
        }
        return { status: "ok" };
    }

    async onSubagentEnded({ targetSessionKey, outcome }: any) {
        if (native.karmaLearnFromSubagent) {
            await native.karmaLearnFromSubagent(targetSessionKey, outcome);
        }
    }

    dispose() {
        if (native.shutdown) {
            native.shutdown();
        }
    }
}
