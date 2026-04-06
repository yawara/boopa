import {
  Badge,
  Button,
  Group,
  Paper,
  Stack,
  Text,
  Title,
} from "@mantine/core";

import type { CacheEntry, DistroId } from "../services/api";

interface CacheStatusCardProps {
  distro: DistroId;
  entries: CacheEntry[];
  isRefreshing: boolean;
  onRefresh: () => void;
}

const statusColor = {
  missing: "orange",
  cached: "teal",
  refreshed: "blue",
} as const;

export function CacheStatusCard({
  distro,
  entries,
  isRefreshing,
  onRefresh,
}: CacheStatusCardProps) {
  return (
    <Paper component="section" p="xl">
      <Stack gap="md">
        <div>
          <Text c="boopaAccent.7" fw={700} fz="xs" lts="0.16em" mb={6} tt="uppercase">
            Cache State
          </Text>
          <Title order={2} size="h3">
            Asset readiness for {distro}
          </Title>
        </div>
        <Button disabled={isRefreshing} variant="gradient" onClick={onRefresh}>
          {isRefreshing ? "Refreshing..." : "Refresh Selected Assets"}
        </Button>
        <Stack gap="sm">
          {entries.map((entry) => (
            <Paper
              key={`${entry.bootMode}-${entry.relativePath}`}
              bg="rgba(255, 255, 255, 0.72)"
              p="md"
              radius="24px"
              shadow="xs"
              withBorder={false}
            >
              <Group align="flex-start" justify="space-between" wrap="wrap">
                <div>
                  <Text fw={700}>{entry.logicalName}</Text>
                  <Text c="slate.7" size="sm">
                    {entry.relativePath}
                  </Text>
                </div>
                <Badge color={statusColor[entry.status]} tt="uppercase">
                  {entry.status}
                </Badge>
              </Group>
            </Paper>
          ))}
        </Stack>
      </Stack>
    </Paper>
  );
}
