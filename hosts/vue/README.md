# uiko Vue host

This is the first browser host for the G0 renderer/read slice.

It deliberately contains no application contract parsing, upstream endpoint selection or OpenAPI logic. It consumes only the public renderer-neutral `UiManifest`, invokes compiled logical query IDs through the same-origin `/__uiko/query` boundary, maps the result through `JsonRenderAdapter`, and renders with the official `@json-render/vue` package.

Generate the fixture manifest from the repository root:

```bash
cargo run -p uiko-cli -- build fixtures/support-console \
  --ui-manifest hosts/vue/public/app.uiko-manifest.json
```

Start the read-only G0 gateway with a deployment-owned base URL for the `crm` provider:

```bash
cargo run -p uiko-g0-gateway -- fixtures/support-console \
  --integration crm=http://127.0.0.1:4010
```

Then start the host:

```bash
cd hosts/vue
npm install
npm run dev
```

The Vite development server proxies only the logical `/__uiko/*` boundary to the local gateway on port 3001. The browser bundle never receives the CRM base URL, OpenAPI path template or HTTP method.

Routes in the current fixture:

- `/customers`
- `/customers/:customerId`

Field and Table values are resolved from logical query results. Filtering and pagination remain outside this M4b slice.
