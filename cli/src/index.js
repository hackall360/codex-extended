#!/usr/bin/env node
const { createHash } = require("crypto");
const { createReadStream, statSync } = require("fs");

const args = process.argv.slice(2);
let host = "http://localhost:3000";
const hIndex = args.indexOf("--host");
if (hIndex !== -1) {
  host = args[hIndex + 1];
  args.splice(hIndex, 2);
}

async function request(path, options) {
  const url = host + path;
  try {
    const res = await fetch(url, options);
    if (!res.ok) {
      console.error(`HTTP ${res.status} ${res.statusText}`);
      process.exit(1);
    }
    return await res.json();
  } catch (err) {
    console.error(`Network error: ${err.message}`);
    process.exit(1);
  }
}

async function modelsList() {
  const data = await request("/models", { method: "GET" });
  console.log(JSON.stringify(data));
}

async function embed(text) {
  const data = await request("/embed", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ text }),
  });
  const embedding = data.embedding || [];
  const buf = Buffer.from(new Float32Array(embedding).buffer);
  const hash = createHash("sha256").update(buf).digest("hex");
  console.log(hash);
}

async function ingest(file) {
  const size = statSync(file).size;
  const stream = createReadStream(file);
  let uploaded = 0;
  stream.on("data", (chunk) => {
    uploaded += chunk.length;
    const percent = Math.round((uploaded / size) * 100);
    process.stdout.write(`\r${percent}%`);
  });
  await request("/ingest", {
    method: "POST",
    headers: { "content-type": "application/octet-stream" },
    body: stream,
    duplex: "half",
  });
  process.stdout.write("\n");
  console.log("ok");
}

async function ask(question) {
  const data = await request("/ask", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ question }),
  });
  console.log(data.answer);
}

async function adminCompact() {
  const data = await request("/admin/compact", { method: "POST" });
  console.log(data.status || "ok");
}

function usage() {
  console.log(`Usage:
  models list
  embed <text>
  ingest <file>
  ask <question>
  admin compact
`);
  process.exit(1);
}

async function main() {
  const cmd = args.shift();
  switch (cmd) {
    case "models":
      if (args.shift() === "list") await modelsList();
      else usage();
      break;
    case "embed":
      if (args.length) await embed(args.join(" "));
      else usage();
      break;
    case "ingest":
      if (args.length) await ingest(args[0]);
      else usage();
      break;
    case "ask":
      if (args.length) await ask(args.join(" "));
      else usage();
      break;
    case "admin":
      if (args.shift() === "compact") await adminCompact();
      else usage();
      break;
    default:
      usage();
  }
}

main();
