import { Route, Routes } from "react-router-dom";

export function App() {
  return (
    <Routes>
      <Route path="*" element={<main>No probe route configured</main>} />
    </Routes>
  );
}
