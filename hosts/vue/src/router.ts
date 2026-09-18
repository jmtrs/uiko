import type { UiRoute } from "./manifest";

export interface RouteMatch {
  route: UiRoute;
  params: Record<string, string>;
}

export function matchRoute(routes: UiRoute[], pathname: string): RouteMatch | null {
  const pathSegments = splitPath(pathname);

  for (const route of routes) {
    const patternSegments = splitPath(route.path);
    if (patternSegments.length !== pathSegments.length) {
      continue;
    }

    const params: Record<string, string> = {};
    let matches = true;

    for (let index = 0; index < patternSegments.length; index += 1) {
      const pattern = patternSegments[index];
      const value = pathSegments[index];

      if (pattern === undefined || value === undefined) {
        matches = false;
        break;
      }

      if (pattern.startsWith(":")) {
        params[pattern.slice(1)] = decodeURIComponent(value);
      } else if (pattern !== value) {
        matches = false;
        break;
      }
    }

    if (matches) {
      return { route, params };
    }
  }

  return null;
}

function splitPath(path: string): string[] {
  return path.split("/").filter(Boolean);
}
