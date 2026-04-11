import {
  Box,
  Paper,
  Stack,
  Typography,
} from "@mui/material";

import type {
  DhcpGuidance,
  DhcpLeaseSummary,
  DhcpResponse,
  DhcpRuntimeStatusResponse,
} from "../services/api";

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

function RuntimeStatusCard({ runtime }: { runtime: DhcpRuntimeStatusResponse }) {
  const leasePreview = runtime.activeLeases.slice(0, 4);

  return (
    <Paper
      sx={{
        bgcolor: "rgba(233, 240, 235, 0.86)",
        p: 3,
        borderRadius: 3.5,
        border: "1px solid rgba(58, 94, 71, 0.12)",
      }}
    >
      <Stack spacing={2.5}>
        <div>
          <Typography color="text.secondary" fontSize="0.75rem" fontWeight={700} letterSpacing="0.12em" textTransform="uppercase">
            DHCP Runtime
          </Typography>
          <Typography component="h3" variant="h6">
            {runtime.enabled ? "Authoritative mode enabled" : "Disabled"}
          </Typography>
        </div>

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
          <RuntimeField label="Mode" value={runtime.mode} />
          <RuntimeField label="Bind" value={runtime.bindAddress} />
          <RuntimeField label="Subnet" value={runtime.subnet ?? "not configured"} />
          <RuntimeField
            label="Pool"
            value={
              runtime.poolStart && runtime.poolEnd
                ? `${runtime.poolStart} - ${runtime.poolEnd}`
                : "not configured"
            }
          />
          <RuntimeField label="Router" value={runtime.router ?? "not configured"} />
          <RuntimeField
            label="Lease Duration"
            value={runtime.leaseDurationSecs ? `${runtime.leaseDurationSecs}s` : "not configured"}
          />
        </Box>

        <Stack spacing={1}>
          <Typography color="text.secondary" fontSize="0.75rem" fontWeight={700} letterSpacing="0.12em" textTransform="uppercase">
            Active Leases
          </Typography>
          <Typography fontWeight={600}>
            {runtime.activeLeaseCount} active lease{runtime.activeLeaseCount === 1 ? "" : "s"}
          </Typography>
          {leasePreview.length > 0 ? (
            <Stack spacing={1}>
              {leasePreview.map((lease) => (
                <LeaseRow key={`${lease.clientKey}-${lease.ipAddress}`} lease={lease} />
              ))}
            </Stack>
          ) : (
            <Typography color="text.secondary" variant="body2">
              No active leases yet.
            </Typography>
          )}
        </Stack>
      </Stack>
    </Paper>
  );
}

function RuntimeField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <Typography color="text.secondary" fontSize="0.75rem" fontWeight={700} letterSpacing="0.12em" textTransform="uppercase">
        {label}
      </Typography>
      <Typography fontWeight={600}>{value}</Typography>
    </div>
  );
}

function LeaseRow({ lease }: { lease: DhcpLeaseSummary }) {
  return (
    <Paper
      sx={{
        bgcolor: "rgba(255, 255, 255, 0.84)",
        p: 1.75,
        borderRadius: 2.5,
        boxShadow: "0 8px 18px rgba(39, 65, 82, 0.06)",
      }}
    >
      <Stack spacing={0.25}>
        <Typography fontWeight={700}>{lease.ipAddress}</Typography>
        <Typography color="text.secondary" variant="body2">
          {lease.clientMac}
        </Typography>
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
            DHCP state for {data.selected}
          </Typography>
        </div>
        <RuntimeStatusCard runtime={data.runtime} />
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
