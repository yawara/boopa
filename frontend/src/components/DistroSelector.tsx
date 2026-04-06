import {
  NativeSelect,
  Paper,
  Stack,
  Text,
  Title,
} from "@mantine/core";

import type { DistroId, DistroSummary } from "../services/api";

interface DistroSelectorProps {
  current: DistroId;
  distros: DistroSummary[];
  isSaving: boolean;
  onChange: (distro: DistroId) => void;
}

export function DistroSelector({
  current,
  distros,
  isSaving,
  onChange,
}: DistroSelectorProps) {
  return (
    <Paper component="section" p="xl">
      <Stack gap="md">
        <div>
          <Text c="boopaAccent.7" fw={700} fz="xs" lts="0.16em" mb={6} tt="uppercase">
            Active Distribution
          </Text>
          <Title order={2} size="h3">
            Choose what the LAN boots next
          </Title>
        </div>
        <NativeSelect
          aria-label="Distro"
          data={distros.map((distro) => ({
            value: distro.id,
            label: distro.label,
          }))}
          label="Distro"
          value={current}
          disabled={isSaving}
          onChange={(event) => onChange(event.target.value as DistroId)}
        />
        <Text c="slate.7">
          The selected distro is persisted by the backend and controls both DHCP guidance and boot
          asset refreshes.
        </Text>
      </Stack>
    </Paper>
  );
}
