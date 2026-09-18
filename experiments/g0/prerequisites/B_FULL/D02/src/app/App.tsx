import { Route, Routes } from "react-router-dom";

import { CustomerDetailPage } from "../features/customers/CustomerDetailPage";

export function App() {
  return (
    <Routes>
      <Route path="/customers/:customerId" element={<CustomerDetailPage />} />
    </Routes>
  );
}
