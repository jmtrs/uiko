import { createServer } from "node:http";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    port: { type: "string", default: "4317" },
    delay: { type: "string", default: process.env.G0_FIXTURE_DELAY_MS ?? "250" },
  },
});

const port = integer(values.port, "port");
const delayMs = integer(values.delay, "delay");
const journal = [];

const customers = buildCustomers();
const customerById = new Map(customers.map((customer) => [customer.id, customer]));
const ordersByCustomer = new Map([
  [
    "cus_ada",
    [
      { id: "ord_ada_1", createdAt: "2026-09-01T10:00:00Z", totalCents: 1299, status: "paid" },
      { id: "ord_ada_2", createdAt: "2026-09-02T11:30:00Z", totalCents: 4200, status: "open" },
    ],
  ],
  [
    "cus_lin",
    [
      { id: "ord_lin_1", createdAt: "2026-09-03T09:15:00Z", totalCents: 875, status: "cancelled" },
    ],
  ],
]);

const server = createServer(async (request, response) => {
  const url = new URL(request.url ?? "/", `http://127.0.0.1:${port}`);
  cors(response);

  if (request.method === "OPTIONS") {
    response.writeHead(204).end();
    return;
  }

  if (request.method === "GET" && url.pathname === "/__harness/health") {
    json(response, 200, { status: "ok" });
    return;
  }

  if (request.method === "POST" && url.pathname === "/__harness/reset") {
    journal.length = 0;
    json(response, 200, { status: "ok" });
    return;
  }

  if (request.method === "GET" && url.pathname === "/__harness/requests") {
    json(response, 200, { requests: journal });
    return;
  }

  if (request.method !== "GET") {
    json(response, 405, { code: "method_not_allowed", message: "GET required" });
    return;
  }

  const entry = {
    sequence: journal.length + 1,
    method: request.method,
    pathname: url.pathname,
    query: Object.fromEntries(url.searchParams.entries()),
  };
  journal.push(entry);

  if (url.pathname === "/customers") {
    await delay(delayMs);
    listCustomers(url, response);
    return;
  }

  const ordersMatch = /^\/customers\/([^/]+)\/orders$/.exec(url.pathname);
  if (ordersMatch !== null) {
    const customerId = decodeURIComponent(ordersMatch[1]);
    json(response, 200, { items: ordersByCustomer.get(customerId) ?? [] });
    return;
  }

  const customerMatch = /^\/customers\/([^/]+)$/.exec(url.pathname);
  if (customerMatch !== null) {
    const customerId = decodeURIComponent(customerMatch[1]);
    const customer = customerById.get(customerId);
    if (customer === undefined) {
      json(response, 404, { code: "customer_not_found", message: "Customer not found" });
    } else {
      json(response, 200, customer);
    }
    return;
  }

  json(response, 404, { code: "not_found", message: "Unknown fixture route" });
});

server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`g0 fixture listening on http://127.0.0.1:${port}\n`);
});

function listCustomers(url, response) {
  const status = url.searchParams.get("status");
  const q = url.searchParams.get("q")?.toLocaleLowerCase();
  const page = positiveInteger(url.searchParams.get("page"), 1);
  const limit = positiveInteger(url.searchParams.get("limit"), 20);

  let filtered = customers;
  if (status === "active" || status === "inactive") {
    filtered = filtered.filter((customer) => customer.status === status);
  }
  if (q !== undefined) {
    filtered = filtered.filter(
      (customer) =>
        customer.name.toLocaleLowerCase().includes(q) ||
        customer.email.toLocaleLowerCase().includes(q),
    );
  }

  const start = (page - 1) * limit;
  json(response, 200, {
    items: filtered.slice(start, start + limit),
    total: filtered.length,
    page,
    pageSize: limit,
  });
}

function buildCustomers() {
  const fixed = [
    {
      id: "cus_ada",
      name: "Ada Lovelace",
      email: "ada@example.test",
      status: "active",
      organization: { id: "org_analytical", name: "Analytical Engines" },
      phone: "+34 600 111 222",
    },
    {
      id: "cus_lin",
      name: "Lin Chen",
      email: "lin@example.test",
      status: "inactive",
      organization: { id: "org_orbit", name: "Orbit Systems" },
      phone: null,
    },
  ];

  const generated = [];
  for (let index = 1; index <= 23; index += 1) {
    generated.push({
      id: `cus_active${index}`,
      name: `Active Customer ${String(index).padStart(2, "0")}`,
      email: `active${index}@example.test`,
      status: "active",
      organization: { id: "org_active", name: "Active Co" },
      phone: `+34 610 00 ${String(index).padStart(3, "0")}`,
    });
  }
  for (let index = 1; index <= 19; index += 1) {
    generated.push({
      id: `cus_inactive${index}`,
      name: `Inactive Customer ${String(index).padStart(2, "0")}`,
      email: `inactive${index}@example.test`,
      status: "inactive",
      organization: { id: "org_inactive", name: "Inactive Co" },
      phone: null,
    });
  }
  return [...fixed, ...generated];
}

function positiveInteger(raw, fallback) {
  if (raw === null) {
    return fallback;
  }
  const value = Number(raw);
  return Number.isInteger(value) && value > 0 ? value : fallback;
}

function integer(raw, name) {
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${name} must be a non-negative integer`);
  }
  return value;
}

function cors(response) {
  response.setHeader("Access-Control-Allow-Origin", "*");
  response.setHeader("Access-Control-Allow-Headers", "Content-Type");
  response.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
}

function json(response, status, body) {
  response.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
  response.end(JSON.stringify(body));
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function shutdown() {
  server.close(() => process.exit(0));
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
