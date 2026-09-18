import {
  Alert,
  CircularProgress,
  Container,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../api/client";

export function CustomerListPage() {
  const customers = useQuery({
    queryKey: ["customers"],
    queryFn: async () => {
      const { data, error } = await api.GET("/customers");
      if (error !== undefined) {
        throw new Error("Unable to load customers");
      }
      return data;
    },
  });

  return (
    <Container component="main" sx={{ py: 4 }}>
      <Typography component="h1" variant="h4" gutterBottom>
        Customers
      </Typography>
      {customers.isPending ? (
        <CircularProgress aria-label="Loading customers" />
      ) : customers.isError ? (
        <Alert severity="error">Unable to load customers</Alert>
      ) : (
        <Table aria-label="Customers">
          <TableHead>
            <TableRow>
              <TableCell>Name</TableCell>
              <TableCell>Email</TableCell>
              <TableCell>Status</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {customers.data.items.map((customer) => (
              <TableRow key={customer.id}>
                <TableCell>{customer.name}</TableCell>
                <TableCell>{customer.email}</TableCell>
                <TableCell>{customer.status}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </Container>
  );
}
