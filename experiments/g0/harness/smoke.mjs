import { chromium } from "@playwright/test";

import { spawnManaged, stopManaged, waitForHttp } from "./processes.mjs";

const child = spawnManaged(
  process.execPath,
  ["fixture-server.mjs", "--port", "4327", "--delay", "0"],
  { cwd: new URL(".", import.meta.url) },
);

try {
  const base = "http://127.0.0.1:4327";
  await waitForHttp(`${base}/__harness/health`);

  const first = await fetch(`${base}/customers?page=1&status=active`);
  assert(first.ok, "active customer page failed");
  const page1 = await first.json();
  assert(page1.page === 1, "page 1 metadata mismatch");
  assert(page1.pageSize === 20, "default pageSize mismatch");
  assert(page1.total === 24, `expected 24 active customers, got ${page1.total}`);
  assert(page1.items.every((customer) => customer.status === "active"), "active filter leaked rows");

  const second = await fetch(`${base}/customers?page=2&status=active`);
  const page2 = await second.json();
  assert(page2.items.length === 4, `expected 4 active rows on page 2, got ${page2.items.length}`);

  const ada = await (await fetch(`${base}/customers/cus_ada`)).json();
  const lin = await (await fetch(`${base}/customers/cus_lin`)).json();
  assert(ada.organization.name === "Analytical Engines", "Ada organization mismatch");
  assert(ada.phone === "+34 600 111 222", "Ada phone mismatch");
  assert(lin.phone === null, "Lin phone must be null");

  const orders = await (await fetch(`${base}/customers/cus_ada/orders`)).json();
  assert(orders.items.length === 2, "Ada orders mismatch");

  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const response = await page.goto(`${base}/__harness/health`);
    assert(response?.ok(), "Chromium could not reach the fixture");
  } finally {
    await browser.close();
  }

  const journal = await (await fetch(`${base}/__harness/requests`)).json();
  assert(journal.requests.length === 5, `expected 5 API requests, got ${journal.requests.length}`);

  await fetch(`${base}/__harness/reset`, { method: "POST" });
  const reset = await (await fetch(`${base}/__harness/requests`)).json();
  assert(reset.requests.length === 0, "journal reset failed");

  process.stdout.write("g0 harness fixture smoke: ok\n");
} finally {
  await stopManaged(child);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
