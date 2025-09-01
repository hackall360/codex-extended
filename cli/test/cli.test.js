const test = require("node:test");
const assert = require("node:assert");
const { createServer } = require("http");
const { execFile } = require("child_process");
const { writeFileSync, unlinkSync } = require("fs");
const { tmpdir } = require("os");
const { join } = require("path");
const crypto = require("crypto");

function startServer() {
  const server = createServer((req, res) => {
    const { method, url } = req;
    if (method === "GET" && url === "/models") {
      res.setHeader("content-type", "application/json");
      res.end(JSON.stringify({ models: ["m1", "m2"] }));
    } else if (method === "POST" && url === "/embed") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        res.setHeader("content-type", "application/json");
        res.end(JSON.stringify({ embedding: [1, 2, 3, 4] }));
      });
    } else if (method === "POST" && url === "/ingest") {
      req.on("data", () => {});
      req.on("end", () => {
        res.setHeader("content-type", "application/json");
        res.end(JSON.stringify({ status: "ok" }));
      });
    } else if (method === "POST" && url === "/ask") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        res.setHeader("content-type", "application/json");
        res.end(JSON.stringify({ answer: "42" }));
      });
    } else if (method === "POST" && url === "/admin/compact") {
      res.setHeader("content-type", "application/json");
      res.end(JSON.stringify({ status: "compacted" }));
    } else {
      res.statusCode = 404;
      res.end();
    }
  });
  return new Promise((resolve) => {
    server.listen(0, () => {
      const { port } = server.address();
      resolve({ server, port });
    });
  });
}

function runCli(port, args) {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      [
        require.resolve("../src/index.js"),
        "--host",
        `http://localhost:${port}`,
        ...args,
      ],
      { encoding: "utf8" },
      (error, stdout, stderr) => {
        const status = error ? error.code || 1 : 0;
        resolve({ stdout, stderr, status });
      },
    );
  });
}

test("models list", async () => {
  const { server, port } = await startServer();
  const out = await runCli(port, ["models", "list"]);
  assert.equal(out.status, 0);
  assert.match(out.stdout, /m1/);
  await new Promise((r) => server.close(r));
});

test("embed checksum", async () => {
  const { server, port } = await startServer();
  const out = await runCli(port, ["embed", "hello"]);
  assert.equal(out.status, 0);
  const buf = Buffer.from(new Float32Array([1, 2, 3, 4]).buffer);
  const expected = crypto.createHash("sha256").update(buf).digest("hex");
  assert.match(out.stdout, new RegExp(expected));
  await new Promise((r) => server.close(r));
});

test("ingest progress", async () => {
  const { server, port } = await startServer();
  const file = join(tmpdir(), "ingest.txt");
  writeFileSync(file, "data");
  const out = await runCli(port, ["ingest", file]);
  assert.equal(out.status, 0);
  assert.match(out.stdout, /100%/);
  unlinkSync(file);
  await new Promise((r) => server.close(r));
});

test("ask", async () => {
  const { server, port } = await startServer();
  const out = await runCli(port, ["ask", "why"]);
  assert.equal(out.status, 0);
  assert.match(out.stdout, /42/);
  await new Promise((r) => server.close(r));
});

test("admin compact", async () => {
  const { server, port } = await startServer();
  const out = await runCli(port, ["admin", "compact"]);
  assert.equal(out.status, 0);
  assert.match(out.stdout, /compacted/);
  await new Promise((r) => server.close(r));
});
