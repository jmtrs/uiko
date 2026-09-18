import { readFile, readdir, stat } from "node:fs/promises";
import { join } from "node:path";

export async function runTaskAcceptance(context) {
  await resetJournal(context.fixtureUrl);
  switch (context.taskId) {
    case "G0-D01":
      return d01(context);
    case "G0-D02":
      return d02(context);
    case "G0-D03":
      return d03(context);
    case "G0-D04":
      return d04(context);
    case "G0-D05":
      return d05(context);
    case "G0-D06":
      return d06(context);
    default:
      throw new Error(`Unknown G0 task ${context.taskId}`);
  }
}

async function d01(ctx) {
  const { page, baseUrl, reporter } = ctx;

  await reporter.browser("navigate /customers", () =>
    page.goto(`${baseUrl}/customers`, { waitUntil: "domcontentloaded" }),
  );

  await reporter.criterion(2, "loading indicator is visible while listCustomers is pending", async () => {
    const loading = page.locator('[role="progressbar"], [role="status"]').first();
    await loading.waitFor({ state: "visible", timeout: 1_500 });
  });

  await waitForText(page, "Ada Lovelace");
  await waitForText(page, "Lin Chen");

  await reporter.criterion(0, "initial render issues listCustomers exactly once", async () => {
    const requests = await journal(ctx.fixtureUrl);
    const listRequests = requests.filter((request) => request.pathname === "/customers");
    assert(listRequests.length === 1, `expected 1 /customers request, got ${listRequests.length}`);
  });

  await reporter.criterion(1, "table renders fixture names, emails and statuses", async () => {
    const table = page.getByRole("table").first();
    const text = await table.innerText();
    for (const expected of [
      "Ada Lovelace",
      "ada@example.test",
      "active",
      "Lin Chen",
      "lin@example.test",
      "inactive",
    ]) {
      assert(text.includes(expected), `table is missing ${expected}`);
    }
  });

  await reporter.criterion(3, "customer fixture values are not hard-coded in measured sources", async () => {
    const source = await measuredSourceText(ctx.repoRoot, ctx.arm);
    for (const forbidden of ["Ada Lovelace", "ada@example.test", "Lin Chen", "lin@example.test"]) {
      assert(!source.includes(forbidden), `measured source hard-codes fixture value ${forbidden}`);
    }
  });
}

async function d02(ctx) {
  const { page, baseUrl, reporter } = ctx;

  await reporter.browser("navigate /customers/cus_ada", () =>
    page.goto(`${baseUrl}/customers/cus_ada`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Ada Lovelace");

  await reporter.criterion(0, "cus_ada route parameter reaches getCustomer", async () => {
    const request = await waitForJournal(
      ctx.fixtureUrl,
      (entry) => entry.pathname === "/customers/cus_ada",
    );
    assert(request !== undefined, "missing getCustomer request for cus_ada");
  });

  await reporter.criterion(1, "Ada detail renders name, email and status", async () => {
    await visibleTexts(page, ["Ada Lovelace", "ada@example.test", "active"]);
  });

  await reporter.browser("navigate /customers/cus_lin", () =>
    page.goto(`${baseUrl}/customers/cus_lin`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Lin Chen");

  await reporter.criterion(2, "route change requests cus_lin and updates the detail", async () => {
    const request = await waitForJournal(
      ctx.fixtureUrl,
      (entry) => entry.pathname === "/customers/cus_lin",
    );
    assert(request !== undefined, "missing getCustomer request for cus_lin");
    await visibleTexts(page, ["Lin Chen", "lin@example.test", "inactive"]);
  });

  await reporter.criterion(3, "route parameter is not duplicated as a hard-coded request value", async () => {
    const source = await measuredSourceText(ctx.repoRoot, ctx.arm);
    assert(!source.includes("cus_ada"), "measured source hard-codes cus_ada");
    assert(!source.includes("cus_lin"), "measured source hard-codes cus_lin");
  });
}

async function d03(ctx) {
  const { page, baseUrl, reporter } = ctx;
  let mainFrameNavigations = 0;
  page.on("framenavigated", (frame) => {
    if (frame === page.mainFrame()) {
      mainFrameNavigations += 1;
    }
  });

  await reporter.browser("navigate /customers", () =>
    page.goto(`${baseUrl}/customers`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Ada Lovelace");
  const initialNavigations = mainFrameNavigations;

  await reporter.criterion(0, "default All omits status", async () => {
    const request = (await journal(ctx.fixtureUrl)).find((entry) => entry.pathname === "/customers");
    assert(request !== undefined, "missing initial listCustomers request");
    assert(!("status" in request.query), "default request unexpectedly contains status");
  });

  await reporter.browser("select Active status", () => chooseStatus(page, "Active"));
  await waitForJournal(
    ctx.fixtureUrl,
    (entry) => entry.pathname === "/customers" && entry.query.status === "active",
  );
  await waitForText(page, "Ada Lovelace");

  await reporter.criterion(1, "Active maps to status=active and updates rows", async () => {
    const text = await page.getByRole("table").first().innerText();
    assert(text.includes("Ada Lovelace"), "active result does not include Ada");
    assert(!text.includes("Lin Chen"), "active result still includes inactive Lin");
  });

  await reporter.browser("select Inactive status", () => chooseStatus(page, "Inactive"));
  await waitForJournal(
    ctx.fixtureUrl,
    (entry) => entry.pathname === "/customers" && entry.query.status === "inactive",
  );
  await waitForText(page, "Lin Chen");

  await reporter.criterion(2, "Inactive maps to status=inactive and updates rows", async () => {
    const text = await page.getByRole("table").first().innerText();
    assert(text.includes("Lin Chen"), "inactive result does not include Lin");
    assert(!text.includes("Ada Lovelace"), "inactive result still includes active Ada");
  });

  await reporter.criterion(3, "filter changes do not cause full-page navigation", async () => {
    assert(
      mainFrameNavigations === initialNavigations,
      `filter caused main-frame navigation: ${initialNavigations} -> ${mainFrameNavigations}`,
    );
  });
}

async function d04(ctx) {
  const { page, baseUrl, reporter } = ctx;

  await reporter.browser("navigate Ada detail", () =>
    page.goto(`${baseUrl}/customers/cus_ada`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Ada Lovelace");

  await reporter.criterion(0, "Organization renders organization.name", async () => {
    await waitForText(page, "Analytical Engines");
  });
  await reporter.criterion(1, "non-null phone renders verbatim", async () => {
    await waitForText(page, "+34 600 111 222");
  });

  await reporter.browser("navigate Lin detail", () =>
    page.goto(`${baseUrl}/customers/cus_lin`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Lin Chen");

  await reporter.criterion(2, "null phone renders Not provided", async () => {
    await waitForText(page, "Not provided");
  });
  await reporter.criterion(3, "both fixture customers render without an error state", async () => {
    await waitForText(page, "Orbit Systems");
    const alerts = page.getByRole("alert");
    assert((await alerts.count()) === 0, "detail view exposes an error alert");
    const body = await page.locator("body").innerText();
    assert(!body.includes("null"), "detail view renders literal null");
  });
}

async function d05(ctx) {
  const { page, baseUrl, reporter } = ctx;

  await reporter.browser("navigate paginated directory", () =>
    page.goto(`${baseUrl}/customers`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Ada Lovelace");

  await reporter.criterion(0, "first listCustomers request uses page=1", async () => {
    const request = (await journal(ctx.fixtureUrl)).find((entry) => entry.pathname === "/customers");
    assert(request?.query.page === "1", `expected page=1, got ${request?.query.page ?? "missing"}`);
  });

  await reporter.criterion(3, "Previous is disabled on page 1", async () => {
    const previous = page.getByRole("button", { name: "Previous" });
    assert(await previous.isDisabled(), "Previous is enabled on page 1");
  });

  await reporter.browser("select Active before paging", () => chooseStatus(page, "Active"));
  await waitForJournal(
    ctx.fixtureUrl,
    (entry) =>
      entry.pathname === "/customers" &&
      entry.query.status === "active" &&
      entry.query.page === "1",
  );

  await reporter.browser("click Next", () =>
    page.getByRole("button", { name: "Next" }).click(),
  );
  await waitForJournal(
    ctx.fixtureUrl,
    (entry) =>
      entry.pathname === "/customers" &&
      entry.query.status === "active" &&
      entry.query.page === "2",
  );

  await reporter.criterion(1, "Next requests page=2 and preserves status=active", async () => {
    const requests = await journal(ctx.fixtureUrl);
    const request = [...requests]
      .reverse()
      .find((entry) => entry.pathname === "/customers");
    assert(request?.query.page === "2", "latest list request is not page=2");
    assert(request?.query.status === "active", "latest list request lost status=active");
  });

  await reporter.criterion(4, "Next is disabled on the final active page", async () => {
    const next = page.getByRole("button", { name: "Next" });
    assert(await next.isDisabled(), "Next is enabled when page * pageSize >= total");
  });

  await reporter.browser("click Previous", () =>
    page.getByRole("button", { name: "Previous" }).click(),
  );
  await waitForJournal(
    ctx.fixtureUrl,
    (entry) =>
      entry.pathname === "/customers" &&
      entry.query.status === "active" &&
      entry.query.page === "1",
    2,
  );

  await reporter.criterion(2, "Previous returns to page=1 while preserving status", async () => {
    const requests = (await journal(ctx.fixtureUrl)).filter(
      (entry) =>
        entry.pathname === "/customers" &&
        entry.query.status === "active" &&
        entry.query.page === "1",
    );
    assert(requests.length >= 2, "Previous did not issue a second active page=1 request");
  });
}

async function d06(ctx) {
  const { page, baseUrl, reporter } = ctx;

  await reporter.browser("navigate Ada customer with orders", () =>
    page.goto(`${baseUrl}/customers/cus_ada`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Ada Lovelace");
  await waitForText(page, "ord_ada_1");

  await reporter.criterion(0, "Ada id is shared by customer and orders requests", async () => {
    const requests = await journal(ctx.fixtureUrl);
    assert(requests.some((entry) => entry.pathname === "/customers/cus_ada"), "missing Ada customer request");
    assert(
      requests.some((entry) => entry.pathname === "/customers/cus_ada/orders"),
      "missing Ada orders request",
    );
  });

  await reporter.criterion(2, "orders table projects response items", async () => {
    const tables = page.getByRole("table");
    let found = false;
    for (let index = 0; index < (await tables.count()); index += 1) {
      const text = await tables.nth(index).innerText();
      if (text.includes("ord_ada_1")) {
        found = true;
        assert(!/^items\b/m.test(text), "orders table appears to render the envelope key items");
      }
    }
    assert(found, "no table contains fixture order rows");
  });

  await reporter.criterion(3, "order id, status and total are visible", async () => {
    await visibleTexts(page, ["ord_ada_1", "paid", "ord_ada_2", "open"]);
    const body = await page.locator("body").innerText();
    assert(
      body.includes("1299") || body.includes("12.99"),
      "first order total is not visible as cents or a formatted decimal",
    );
    assert(
      body.includes("4200") || body.includes("42.00") || body.includes("42"),
      "second order total is not visible as cents or a formatted decimal",
    );
  });

  await reporter.browser("navigate Lin customer with orders", () =>
    page.goto(`${baseUrl}/customers/cus_lin`, { waitUntil: "domcontentloaded" }),
  );
  await waitForText(page, "Lin Chen");
  await waitForText(page, "ord_lin_1");

  await reporter.criterion(1, "route change updates both customer and orders requests to Lin", async () => {
    const requests = await journal(ctx.fixtureUrl);
    assert(requests.some((entry) => entry.pathname === "/customers/cus_lin"), "missing Lin customer request");
    assert(
      requests.some((entry) => entry.pathname === "/customers/cus_lin/orders"),
      "missing Lin orders request",
    );
  });
}

async function chooseStatus(page, label) {
  const combobox = page.getByRole("combobox", { name: "Status" });
  await combobox.waitFor({ state: "visible" });
  const tagName = await combobox.evaluate((element) => element.tagName);
  if (tagName === "SELECT") {
    await combobox.selectOption({ label });
    return;
  }
  await combobox.click();
  await page.getByRole("option", { name: label, exact: true }).click();
}

async function resetJournal(fixtureUrl) {
  const response = await fetch(`${fixtureUrl}/__harness/reset`, { method: "POST" });
  assert(response.ok, "failed to reset fixture journal");
}

async function journal(fixtureUrl) {
  const response = await fetch(`${fixtureUrl}/__harness/requests`);
  assert(response.ok, "failed to read fixture journal");
  return (await response.json()).requests;
}

async function waitForJournal(fixtureUrl, predicate, minimumMatches = 1, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const requests = await journal(fixtureUrl);
    const matches = requests.filter(predicate);
    if (matches.length >= minimumMatches) {
      return matches.at(-1);
    }
    await sleep(50);
  }
  throw new Error("timed out waiting for expected fixture request");
}

async function waitForText(page, text) {
  await page.getByText(text, { exact: true }).first().waitFor({ state: "visible", timeout: 5_000 });
}

async function visibleTexts(page, texts) {
  for (const text of texts) {
    await waitForText(page, text);
  }
}

async function measuredSourceText(repoRoot, arm) {
  const roots =
    arm === "B_FULL"
      ? [join(repoRoot, "baselines/b-full/src")]
      : arm === "R_RENDER_ONLY"
        ? [
            join(repoRoot, "experiments/controls/r-render-only/src"),
            join(repoRoot, "experiments/controls/r-render-only/ui"),
          ]
        : arm === "T_AUTHORING"
          ? [join(repoRoot, "experiments/controls/t-authoring/app.ts")]
          : [
            join(repoRoot, "fixtures/support-console/uiko.jsonc"),
            join(repoRoot, "fixtures/support-console/features"),
          ];
  const files = [];
  for (const root of roots) {
    try {
      await collectFiles(root, files);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw error;
      }
    }
  }
  const texts = await Promise.all(files.map((file) => readFile(file, "utf8")));
  return texts.join("\n");
}

async function collectFiles(path, output) {
  const info = await stat(path);
  if (info.isFile()) {
    output.push(path);
    return;
  }
  for (const entry of await readdir(path)) {
    await collectFiles(join(path, entry), output);
  }
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
