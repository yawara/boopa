import type { ReactNode } from "react";
import {
  Box,
  Container,
  Image,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";

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
      <Paper p="xl">
        <Text size="lg">Loading dashboard...</Text>
      </Paper>
    );
  } else {
    const distrosData = distrosQuery.data;
    if (!selected || !distrosData) {
      throw new Error("DashboardPage rendered ready state without distro data");
    }

    dashboardContent = (
      <>
        <SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg">
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
            <Paper p="xl">
              <Text size="lg">Loading cache state...</Text>
            </Paper>
          )}
        </SimpleGrid>

        {dhcpQuery.data ? (
          <DhcpGuideCard data={dhcpQuery.data} />
        ) : (
          <Paper p="xl">
            <Text size="lg">Loading DHCP guidance...</Text>
          </Paper>
        )}

        {selected === "ubuntu" ? <AutoinstallCard /> : null}
      </>
    );
  }

  return (
    <Box component="main" className="app-shell">
      <Container size="lg" py={{ base: "xl", md: "3rem" }}>
        <Stack gap="xl">
          <Paper className="hero-panel" p={{ base: "xl", md: "2rem" }}>
            <Stack gap="lg">
              <Image
                src={boopaLogo}
                alt="boopa logo"
                className="hero-logo"
                fit="contain"
                w="100%"
                maw={620}
              />
              <Stack gap="xs">
                <Text c="boopaAccent.7" fw={700} fz="xs" lts="0.18em" tt="uppercase">
                  Trusted LAN Control Plane
                </Text>
                <Title order={1} size="h4">
                  boopa
                </Title>
                <Text c="slate.7" maw={720} size="lg">
                  Switch the active distro, inspect the DHCP values the network needs, and verify
                  that the next BIOS and UEFI boots have cached assets ready.
                </Text>
              </Stack>
            </Stack>
          </Paper>
          {dashboardContent}
        </Stack>
      </Container>
    </Box>
  );
}
