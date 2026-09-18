import { Routes, Route } from "react-router-dom";

import { CustomerListPage } from "../features/customers/CustomerListPage";

export function App() {
  return (
    <Routes>
      <Route path="/customers" element={<CustomerListPage />} />
    </Routes>
  );
}
