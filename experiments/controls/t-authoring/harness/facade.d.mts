export type StateValue = string | number | null;

export interface QuerySpec {
  operation: string;
  input?: Record<string, string>;
}

export interface TextComponent {
  id: string;
  type: "Text";
  value: string;
}

export interface FieldComponent {
  id: string;
  type: "Field";
  label: string;
  binding: string;
  fallback?: string;
}

export interface TableComponent {
  id: string;
  type: "Table";
  binding: string;
}

export interface SelectComponent {
  id: string;
  type: "Select";
  label: string;
  state: string;
  options: Array<{ label: string; value: StateValue }>;
}

export interface PaginationComponent {
  id: string;
  type: "Pagination";
  state: string;
  page: string;
  pageSize: string;
  total: string;
}

export type ComponentSpec =
  | TextComponent
  | FieldComponent
  | TableComponent
  | SelectComponent
  | PaginationComponent;

export interface PageSpec {
  file: string;
  id: string;
  route: string;
  state?: Record<string, StateValue>;
  queries?: Record<string, QuerySpec>;
  components?: ComponentSpec[];
}

export interface ModuleSpec {
  file: string;
  id: string;
  pages: PageSpec[];
}

export interface ProjectSpec {
  name: string;
  specVersion: 1;
  modules: ModuleSpec[];
}
