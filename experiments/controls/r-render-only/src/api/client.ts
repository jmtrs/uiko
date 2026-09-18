import createClient from "openapi-fetch";

import type { paths } from "./generated/schema";

const baseUrl = import.meta.env.VITE_API_BASE_URL ?? "";

export const api = createClient<paths>({ baseUrl });
