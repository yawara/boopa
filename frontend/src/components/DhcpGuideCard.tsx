import {
  Box,
  Paper,
  Stack,
  Typography,
} from "@mui/material";

import type { DhcpGuidance, DhcpResponse } from "../services/api";

function ModeGuide({ mode, guide }: { mode: string; guide: DhcpGuidance }) {
  return (
    <Paper sx={{ bgcolor: "rgba(244, 247, 249, 0.9)", p: 3, borderRadius: 3.5 }}>
      <Stack spacing={3}>
        <Box
          sx={{
            display: "flex",
            alignItems: "baseline",
            justifyContent: "space-between",
            gap: 1,
            flexWrap: "wrap",
          }}
        >
          <Typography component="h3" variant="h6">
            {mode}
          </Typography>
          <Typography color="text.secondary" fontWeight={600}>
            {guide.architecture}
          </Typography>
        </Box>

        <Box
          sx={{
            display: "grid",
            gap: 2,
            gridTemplateColumns: {
              xs: "minmax(0, 1fr)",
              sm: "repeat(2, minmax(0, 1fr))",
            },
          }}
        >
          <div>
            <Typography
              color="text.secondary"
              fontSize="0.75rem"
              fontWeight={700}
              letterSpacing="0.12em"
              textTransform="uppercase"
            >
              Boot filename
            </Typography>
            <Typography fontWeight={600}>{guide.bootFilename}</Typography>
          </div>
          <div>
            <Typography
              color="text.secondary"
              fontSize="0.75rem"
              fontWeight={700}
              letterSpacing="0.12em"
              textTransform="uppercase"
            >
              Next server
            </Typography>
            <Typography fontWeight={600}>{guide.nextServer}</Typography>
          </div>
        </Box>

        <Stack spacing={1}>
          {guide.notes.map((note) => (
            <Typography key={note} color="text.secondary" variant="body2">
              • {note}
            </Typography>
          ))}
        </Stack>

        <Stack spacing={2}>
          {guide.options.map((option) => (
            <Paper
              key={`${mode}-${option.key}`}
              sx={{
                bgcolor: "rgba(255, 255, 255, 0.86)",
                p: 2,
                borderRadius: 3,
                border: "none",
                boxShadow: "0 10px 24px rgba(39, 65, 82, 0.08)",
              }}
            >
              <Stack spacing={0.75}>
                <Typography fontWeight={700}>{option.key}</Typography>
                <Box
                  component="code"
                  sx={{
                    bgcolor: "rgba(39, 65, 82, 0.08)",
                    borderRadius: 1.5,
                    fontFamily: "ui-monospace, SFMono-Regular, SF Mono, Menlo, monospace",
                    fontSize: "0.85rem",
                    px: 1,
                    py: 0.5,
                  }}
                >
                  {option.value}
                </Box>
                <Typography color="text.secondary" variant="body2">
                  {option.description}
                </Typography>
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
    <Paper component="section" sx={{ p: 4 }}>
      <Stack spacing={3}>
        <div>
          <Typography
            color="primary.dark"
            fontSize="0.75rem"
            fontWeight={700}
            letterSpacing="0.16em"
            mb={0.75}
            textTransform="uppercase"
          >
            DHCP Guide
          </Typography>
          <Typography component="h2" variant="h5">
            Manual settings for {data.selected}
          </Typography>
        </div>
        <Box
          sx={{
            display: "grid",
            gap: 2,
            gridTemplateColumns: {
              xs: "minmax(0, 1fr)",
              md: "repeat(2, minmax(0, 1fr))",
            },
          }}
        >
          <ModeGuide mode="BIOS" guide={data.bios} />
          <ModeGuide mode="UEFI" guide={data.uefi} />
        </Box>
      </Stack>
    </Paper>
  );
}
