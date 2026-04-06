import {
  Code,
  Group,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";

import type { DhcpGuidance, DhcpResponse } from "../services/api";

function ModeGuide({ mode, guide }: { mode: string; guide: DhcpGuidance }) {
  return (
    <Paper bg="rgba(244, 247, 249, 0.9)" p="lg" radius="28px">
      <Stack gap="md">
        <Group align="baseline" justify="space-between" wrap="wrap">
          <Title order={3} size="h4">
            {mode}
          </Title>
          <Text c="slate.7" fw={600}>
            {guide.architecture}
          </Text>
        </Group>

        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md">
          <div>
            <Text c="slate.6" fw={700} fz="xs" lts="0.12em" tt="uppercase">
              Boot filename
            </Text>
            <Text fw={600}>{guide.bootFilename}</Text>
          </div>
          <div>
            <Text c="slate.6" fw={700} fz="xs" lts="0.12em" tt="uppercase">
              Next server
            </Text>
            <Text fw={600}>{guide.nextServer}</Text>
          </div>
        </SimpleGrid>

        <Stack gap={8}>
          {guide.notes.map((note) => (
            <Text key={note} c="slate.7" size="sm">
              • {note}
            </Text>
          ))}
        </Stack>

        <Stack gap="sm">
          {guide.options.map((option) => (
            <Paper
              key={`${mode}-${option.key}`}
              bg="rgba(255, 255, 255, 0.86)"
              p="md"
              radius="22px"
              shadow="xs"
              withBorder={false}
            >
              <Stack gap={6}>
                <Text fw={700}>{option.key}</Text>
                <Code>{option.value}</Code>
                <Text c="slate.7" size="sm">
                  {option.description}
                </Text>
              </Stack>
            </Paper>
          ))}
        </Stack>
      </Stack>
    </Paper>
  );
}

export function DhcpGuideCard({ data }: { data: DhcpResponse }) {
  return (
    <Paper component="section" p="xl">
      <Stack gap="md">
        <div>
          <Text c="boopaAccent.7" fw={700} fz="xs" lts="0.16em" mb={6} tt="uppercase">
            DHCP Guide
          </Text>
          <Title order={2} size="h3">
            Manual settings for {data.selected}
          </Title>
        </div>
        <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
        <ModeGuide mode="BIOS" guide={data.bios} />
        <ModeGuide mode="UEFI" guide={data.uefi} />
        </SimpleGrid>
      </Stack>
    </Paper>
  );
}
