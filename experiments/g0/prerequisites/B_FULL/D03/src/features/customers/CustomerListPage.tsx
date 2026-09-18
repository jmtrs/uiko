import {
  Alert,
  CircularProgress,
  Container,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../api/client";

type StatusFilter = "all" | "active" | "inactive";

export function CustomerListPage() {
  const [status, setStatus] = useState<StatusFilter>("all");
  const customers = useQuery({
    queryKey: ["customers", status],
    queryFn: async () => {
      const query = status === "all" ? {} : { status };
      const { data, error } = await api.GET("/customers", {
        params: { query },
      });
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
      <FormControl sx={{ mb: 2, minWidth: 180 }}>
        <InputLabel id="status-filter-label">Status</InputLabel>
        <Select
          labelId="status-filter-label"
          label="Status"
          value={status}
          onChange={(event) => setStatus(event.target.value as StatusFilter)}
        >
          <MenuItem value="all">All</MenuItem>
          <MenuItem value="active">Active</MenuItem>
          <MenuItem value="inactive">Inactive</MenuItem>
        </Select>
      </FormControl>
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
