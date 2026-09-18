# uiko Vue host

This is the first browser host for the G0 renderer slice.

It deliberately contains no application contract parsing, endpoint selection or OpenAPI logic. It consumes only the public renderer-neutral `UiManifest`, maps it through `JsonRenderAdapter`, and renders it with the official `@json-render/vue` package.

Generate the fixture manifest from the repository root:

```bash
cargo run -p uiko-cli -- build fixtures/support-console \
  --ui-manifest hosts/vue/public/app.uiko-manifest.json
```

Then:

```bash
cd hosts/vue
npm install
npm run dev
```

Routes in the current fixture:

- `/customers`
- `/customers/:customerId`

The Field and Table components are intentionally shells in M3. Data resolution, list projection, filtering and pagination belong to the next OpenAPI read slice rather than being invented in browser code.
