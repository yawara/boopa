import type { ReactNode } from "react";
import {
  Box,
  Container,
  Paper,
  Stack,
  Typography,
} from "@mui/material";

import { DistroSelector } from "../components/DistroSelector";
import { AutoinstallCard } from "../components/AutoinstallCard";
import { CacheStatusCard } from "../components/CacheStatusCard";
import { DhcpGuideCard } from "../components/DhcpGuideCard";
import boopaLogo from "../assets/boopa-logo.png";
import {
  useGetCacheQuery,
  useGetDhcpQuery,
  useGetDistrosQuery,
  useRefreshCacheMutation,
  useSetSelectionMutation,
} from "../services/api";

export function DashboardPage() {
  const distrosQuery = useGetDistrosQuery();
  const selected = distrosQuery.data?.selected;
  const dhcpQuery = useGetDhcpQuery(selected);
  const cacheQuery = useGetCacheQuery();
  const [setSelection, setSelectionState] = useSetSelectionMutation();
  const [refreshCache, refreshState] = useRefreshCacheMutation();

  const isLoading = distrosQuery.isLoading || !selected || !distrosQuery.data;
  let dashboardContent: ReactNode;

  if (isLoading) {
    dashboardContent = (
      <Paper sx={{ p: 4 }}>
        <Typography variant="h6">Loading dashboard...</Typography>
      </Paper>
    );
  } else {
    const distrosData = distrosQuery.data;
    if (!selected || !distrosData) {
      throw new Error("DashboardPage rendered ready state without distro data");
    }

    dashboardContent = (
      <>
        <Box
          sx={{
            display: "grid",
            gap: 3,
            gridTemplateColumns: {
              xs: "minmax(0, 1fr)",
              md: "repeat(2, minmax(0, 1fr))",
            },
          }}
        >
          <DistroSelector
            current={selected}
            distros={distrosData.distros}
            isSaving={setSelectionState.isLoading}
            onChange={(distro) => {
              void setSelection(distro);
            }}
          />
          {cacheQuery.data ? (
            <CacheStatusCard
              distro={cacheQuery.data.selected}
              entries={cacheQuery.data.entries}
              isRefreshing={refreshState.isLoading}
              onRefresh={() => {
                void refreshCache(selected);
              }}
            />
          ) : (
            <Paper sx={{ p: 4 }}>
              <Typography variant="h6">Loading cache state...</Typography>
            </Paper>
          )}
        </Box>

        {dhcpQuery.data ? (
          <DhcpGuideCard data={dhcpQuery.data} />
        ) : (
          <Paper sx={{ p: 4 }}>
            <Typography variant="h6">Loading DHCP guidance...</Typography>
          </Paper>
        )}

        {selected === "ubuntu" ? <AutoinstallCard /> : null}
      </>
    );
  }

  return (
    <Box component="main" className="app-shell">
      <Container maxWidth="lg" sx={{ py: { xs: 6, md: 8 } }}>
        <Stack spacing={4}>
          <Paper className="hero-panel" sx={{ p: { xs: 4, md: 5 } }}>
            <Stack spacing={3}>
              <Box className="hero-logo">
                <Box
                  component="img"
                  src={boopaLogo}
                  alt="boopa logo"
                  sx={{
                    display: "block",
                    width: "100%",
                    maxWidth: 620,
                    height: "auto",
                    objectFit: "contain",
                  }}
                />
              </Box>
              <Stack spacing={1}>
                <Typography
                  className="hero-kicker"
                  color="primary.dark"
                  fontSize="0.75rem"
                  fontWeight={700}
                  letterSpacing="0.18em"
                  textTransform="uppercase"
                >
                  Trusted LAN Control Plane
                </Typography>
                <Typography className="hero-title" component="h1" variant="h4">
                  boopa
                </Typography>
                <Typography color="text.secondary" maxWidth={720} variant="h6">
                  Switch the active distro, inspect the DHCP values the network needs, and verify
                  that the next BIOS and UEFI boots have cached assets ready.
                </Typography>
              </Stack>
            </Stack>
          </Paper>
          {dashboardContent}
        </Stack>
      </Container>
    </Box>
  );
}
