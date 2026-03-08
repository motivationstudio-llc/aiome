const native = require('./index.darwin-arm64.node');

async function test() {
  console.log("Testing native bindings...");
  
  await native.watchtowerInit();
  console.log("Watchtower Initialized");

  const ts = new Date().toISOString();
  await native.karmaIngest("test_session_1", JSON.stringify({ role: "user", content: "hello world " + ts }));
  console.log("Ingested message");

  const summary = await native.karmaFetchRelevant("test_session_1", 5);
  console.log("Relevant summary:", summary);

  try {
    await native.immuneScanInput("I want to run rm -rf /", "[]");
    console.log("Scan passed (unexpected!)");
  } catch (e) {
    console.log("Scan correctly blocked:", e.message);
  }

  await native.watchtowerTrackUsage(JSON.stringify({ tokens: 100 }));
  console.log("Tracked usage");

  native.watchtowerShutdown();
  console.log("Shutdown complete");
}

test().catch(console.error);
