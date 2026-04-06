import {
  FormControl,
  InputLabel,
  NativeSelect,
  Paper,
  Stack,
  Typography,
} from "@mui/material";

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
            Active Distribution
          </Typography>
          <Typography component="h2" variant="h5">
            Choose what the LAN boots next
          </Typography>
        </div>
        <FormControl fullWidth variant="standard">
          <InputLabel shrink htmlFor="distro-select">
            Distro
          </InputLabel>
          <NativeSelect
            disabled={isSaving}
            value={current}
            onChange={(event) => onChange(event.target.value as DistroId)}
            inputProps={{ id: "distro-select" }}
          >
            {distros.map((distro) => (
              <option key={distro.id} value={distro.id}>
                {distro.label}
              </option>
            ))}
          </NativeSelect>
        </FormControl>
        <Typography color="text.secondary">
          The selected distro is persisted by the backend and controls both DHCP guidance and boot
          asset refreshes.
        </Typography>
      </Stack>
    </Paper>
  );
}
