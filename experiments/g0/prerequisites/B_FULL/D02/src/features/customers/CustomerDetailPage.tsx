import { Alert, CircularProgress, Container, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { api } from "../../api/client";

export function CustomerDetailPage() {
  const { customerId } = useParams<{ customerId: string }>();
  const customer = useQuery({
    queryKey: ["customer", customerId],
    enabled: customerId !== undefined,
    queryFn: async () => {
      if (customerId === undefined) {
        throw new Error("Missing customer id");
      }
      const { data, error } = await api.GET("/customers/{customerId}", {
        params: { path: { customerId } },
      });
      if (error !== undefined) {
        throw new Error("Unable to load customer");
      }
      return data;
    },
  });

  if (customer.isPending) {
    return <CircularProgress aria-label="Loading customer" />;
  }
  if (customer.isError || customer.data === undefined) {
    return <Alert severity="error">Unable to load customer</Alert>;
  }

  return (
    <Container component="main" sx={{ py: 4 }}>
      <Typography component="h1" variant="h4" gutterBottom>
        Customer
      </Typography>
      <Stack spacing={1}>
        <div><strong>Name</strong> {customer.data.name}</div>
        <div><strong>Email</strong> {customer.data.email}</div>
        <div><strong>Status</strong> {customer.data.status}</div>
      </Stack>
    </Container>
  );
}
