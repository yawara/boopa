import {
  Box,
  Button,
  Chip,
  Paper,
  Stack,
  Typography,
} from "@mui/material";

import type { CacheEntry, DistroId } from "../services/api";

interface CacheStatusCardProps {
  distro: DistroId;
  entries: CacheEntry[];
  isRefreshing: boolean;
  onRefresh: () => void;
}

const statusColor = {
  missing: "warning",
  cached: "success",
  refreshed: "info",
} as const;

export function CacheStatusCard({
  distro,
  entries,
  isRefreshing,
  onRefresh,
}: CacheStatusCardProps) {
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
            Cache State
          </Typography>
          <Typography component="h2" variant="h5">
            Asset readiness for {distro}
          </Typography>
        </div>
        <Button disabled={isRefreshing} variant="contained" onClick={onRefresh}>
          {isRefreshing ? "Refreshing..." : "Refresh Selected Assets"}
        </Button>
        <Stack spacing={2}>
          {entries.map((entry) => (
            <Paper
              key={`${entry.bootMode}-${entry.relativePath}`}
              sx={{
                p: 2,
                borderRadius: 3,
                bgcolor: "rgba(255, 255, 255, 0.72)",
                border: "none",
                boxShadow: "0 10px 24px rgba(39, 65, 82, 0.08)",
              }}
            >
              <Box
                sx={{
                  display: "flex",
                  alignItems: "flex-start",
                  justifyContent: "space-between",
                  gap: 2,
                  flexWrap: "wrap",
                }}
              >
                <div>
                  <Typography fontWeight={700}>{entry.logicalName}</Typography>
                  <Typography color="text.secondary" variant="body2">
                    {entry.relativePath}
                  </Typography>
                </div>
                <Chip color={statusColor[entry.status]} label={entry.status} size="small" />
              </Box>
            </Paper>
          ))}
        </Stack>
      </Stack>
    </Paper>
  );
}
