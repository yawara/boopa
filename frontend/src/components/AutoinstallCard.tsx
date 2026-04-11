import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  FormControl,
  FormControlLabel,
  InputLabel,
  NativeSelect,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";

import type {
  UbuntuAutoinstallConfigUpdate,
  UbuntuAutoinstallResponse,
  ValidationErrorResponse,
} from "../services/api";
import {
  useGetUbuntuAutoinstallQuery,
  useSetUbuntuAutoinstallMutation,
} from "../services/api";

interface FormState {
  hostname: string;
  username: string;
  password: string;
  confirmPassword: string;
  locale: string;
  keyboardLayout: string;
  timezone: string;
  storageLayout: "direct" | "lvm";
  installOpenSsh: boolean;
  allowPasswordAuth: boolean;
  authorizedKeysText: string;
  packagesText: string;
}

export function AutoinstallCard() {
  const query = useGetUbuntuAutoinstallQuery();
  const [saveConfig, saveState] = useSetUbuntuAutoinstallMutation();
  const [form, setForm] = useState<FormState | null>(null);
  const [serverMessage, setServerMessage] = useState<string | null>(null);
  const [serverFieldErrors, setServerFieldErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!query.data) {
      return;
    }

    setForm(formFromResponse(query.data));
    setServerMessage(null);
    setServerFieldErrors({});
  }, [query.data]);

  if (query.isLoading || !form || !query.data) {
    return (
      <Paper component="section" sx={{ p: 4 }}>
        <Typography variant="h6">Loading autoinstall config...</Typography>
      </Paper>
    );
  }

  const clientErrors = validateForm(form, query.data.hasPassword);
  const fieldErrors = { ...serverFieldErrors, ...clientErrors };
  const isDirty = formIsDirty(form, query.data);
  const saveDisabled = saveState.isLoading || !isDirty || Object.keys(clientErrors).length > 0;

  async function handleSave() {
    if (!form || saveDisabled) {
      return;
    }

    setServerMessage(null);
    setServerFieldErrors({});

    try {
      await saveConfig(toUpdatePayload(form)).unwrap();
    } catch (error) {
      const validation = parseValidationError(error);
      setServerMessage(validation.message);
      setServerFieldErrors(validation.fieldErrors);
    }
  }

  function updateField<Key extends keyof FormState>(key: Key, value: FormState[Key]) {
    setForm((current) => {
      if (!current) {
        return current;
      }

      return {
        ...current,
        [key]: value,
      };
    });
    setServerMessage(null);
    setServerFieldErrors({});
  }

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
            Autoinstall
          </Typography>
          <Typography component="h2" variant="h5">
            Ubuntu autoinstall defaults
          </Typography>
          <Typography color="text.secondary" mt={1}>
            Save the basic install options the backend will render into
            <Box component="span" sx={{ fontWeight: 700 }}>
              {" "}
              `/boot/ubuntu/uefi/autoinstall/user-data`
            </Box>
            .
          </Typography>
        </div>

        {serverMessage ? <Alert severity="error">{serverMessage}</Alert> : null}

        <Box
          sx={{
            display: "grid",
            gap: 3,
            gridTemplateColumns: {
              xs: "minmax(0, 1fr)",
              xl: "repeat(2, minmax(0, 1fr))",
            },
          }}
        >
          <Paper sx={{ bgcolor: "rgba(244, 247, 249, 0.9)", p: 3, borderRadius: 3.5 }}>
            <Stack spacing={3}>
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
                <TextField
                  label="Hostname"
                  value={form.hostname}
                  error={Boolean(fieldErrors.hostname)}
                  helperText={fieldErrors.hostname}
                  onChange={(event) => updateField("hostname", event.currentTarget.value)}
                />
                <TextField
                  label="Username"
                  value={form.username}
                  error={Boolean(fieldErrors.username)}
                  helperText={fieldErrors.username}
                  onChange={(event) => updateField("username", event.currentTarget.value)}
                />
                <TextField
                  label="Password"
                  type="password"
                  value={form.password}
                  error={Boolean(fieldErrors.password)}
                  helperText={
                    fieldErrors.password ??
                    (query.data.hasPassword
                      ? "Leave blank to keep the saved password hash."
                      : "Required for the first save.")
                  }
                  onChange={(event) => updateField("password", event.currentTarget.value)}
                />
                <TextField
                  label="Confirm password"
                  type="password"
                  value={form.confirmPassword}
                  error={Boolean(fieldErrors.confirmPassword)}
                  helperText={fieldErrors.confirmPassword}
                  onChange={(event) => updateField("confirmPassword", event.currentTarget.value)}
                />
                <TextField
                  label="Locale"
                  value={form.locale}
                  error={Boolean(fieldErrors.locale)}
                  helperText={fieldErrors.locale}
                  onChange={(event) => updateField("locale", event.currentTarget.value)}
                />
                <TextField
                  label="Keyboard layout"
                  value={form.keyboardLayout}
                  error={Boolean(fieldErrors.keyboardLayout)}
                  helperText={fieldErrors.keyboardLayout}
                  onChange={(event) => updateField("keyboardLayout", event.currentTarget.value)}
                />
                <TextField
                  label="Timezone"
                  value={form.timezone}
                  error={Boolean(fieldErrors.timezone)}
                  helperText={fieldErrors.timezone}
                  onChange={(event) => updateField("timezone", event.currentTarget.value)}
                />
                <FormControl fullWidth variant="standard">
                  <InputLabel shrink htmlFor="storage-layout-select">
                    Storage layout
                  </InputLabel>
                  <NativeSelect
                    value={form.storageLayout}
                    onChange={(event) =>
                      updateField("storageLayout", event.currentTarget.value as "direct" | "lvm")
                    }
                    inputProps={{ id: "storage-layout-select" }}
                  >
                    <option value="direct">Direct</option>
                    <option value="lvm">LVM</option>
                  </NativeSelect>
                </FormControl>
              </Box>

              <FormControlLabel
                control={
                  <Checkbox
                    checked={form.installOpenSsh}
                    onChange={(event) => updateField("installOpenSsh", event.currentTarget.checked)}
                  />
                }
                label="Install OpenSSH server"
              />
              <FormControlLabel
                control={
                  <Checkbox
                    checked={form.allowPasswordAuth}
                    onChange={(event) =>
                      updateField("allowPasswordAuth", event.currentTarget.checked)
                    }
                  />
                }
                label="Allow password authentication"
              />

              <TextField
                label="Authorized keys"
                helperText={fieldErrors.authorizedKeys ?? "One SSH public key per line."}
                multiline
                minRows={4}
                value={form.authorizedKeysText}
                error={Boolean(fieldErrors.authorizedKeys)}
                onChange={(event) => updateField("authorizedKeysText", event.currentTarget.value)}
              />
              <TextField
                label="Packages"
                helperText={fieldErrors.packages ?? "One apt package name per line."}
                multiline
                minRows={4}
                value={form.packagesText}
                error={Boolean(fieldErrors.packages)}
                onChange={(event) => updateField("packagesText", event.currentTarget.value)}
              />

              <Button
                disabled={saveDisabled}
                variant="contained"
                onClick={() => {
                  void handleSave();
                }}
              >
                {saveState.isLoading ? "Saving..." : "Save Autoinstall Config"}
              </Button>
            </Stack>
          </Paper>

          <Paper
            sx={{
              bgcolor: "rgba(16, 24, 32, 0.96)",
              color: "common.white",
              p: 3,
              borderRadius: 3.5,
            }}
          >
            <Stack spacing={1.5}>
              <div>
                <Typography
                  color="primary.light"
                  fontSize="0.75rem"
                  fontWeight={700}
                  letterSpacing="0.16em"
                  textTransform="uppercase"
                >
                  YAML Preview
                </Typography>
                <Typography color="common.white" component="h3" mt={0.75} variant="h6">
                  Rendered user-data
                </Typography>
              </div>
              <Typography color="rgba(255,255,255,0.72)" variant="body2">
                This preview reflects the last config saved to the backend.
              </Typography>
              <Box
                component="pre"
                sx={{
                  bgcolor: "rgba(255,255,255,0.04)",
                  color: "common.white",
                  p: 2,
                  fontFamily: "ui-monospace, SFMono-Regular, SF Mono, Menlo, monospace",
                  fontSize: "0.85rem",
                  lineHeight: 1.55,
                  overflowX: "auto",
                  whiteSpace: "pre",
                }}
              >
                {query.data.renderedYaml}
              </Box>
            </Stack>
          </Paper>
        </Box>
      </Stack>
    </Paper>
  );
}

function formFromResponse(response: UbuntuAutoinstallResponse): FormState {
  return {
    hostname: response.config.hostname,
    username: response.config.username,
    password: "",
    confirmPassword: "",
    locale: response.config.locale,
    keyboardLayout: response.config.keyboardLayout,
    timezone: response.config.timezone,
    storageLayout: response.config.storageLayout,
    installOpenSsh: response.config.installOpenSsh,
    allowPasswordAuth: response.config.allowPasswordAuth,
    authorizedKeysText: response.config.authorizedKeys.join("\n"),
    packagesText: response.config.packages.join("\n"),
  };
}

function toUpdatePayload(form: FormState): UbuntuAutoinstallConfigUpdate {
  return {
    hostname: form.hostname.trim(),
    username: form.username.trim(),
    password: form.password.trim() ? form.password : undefined,
    locale: form.locale.trim(),
    keyboardLayout: form.keyboardLayout.trim(),
    timezone: form.timezone.trim(),
    storageLayout: form.storageLayout,
    installOpenSsh: form.installOpenSsh,
    allowPasswordAuth: form.allowPasswordAuth,
    authorizedKeys: linesToList(form.authorizedKeysText),
    packages: linesToList(form.packagesText),
  };
}

function linesToList(value: string): string[] {
  return value
    .split("\n")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function formIsDirty(form: FormState, response: UbuntuAutoinstallResponse): boolean {
  const payload = toUpdatePayload(form);
  const config = response.config;

  return (
    payload.hostname !== config.hostname ||
    payload.username !== config.username ||
    payload.password !== undefined ||
    payload.locale !== config.locale ||
    payload.keyboardLayout !== config.keyboardLayout ||
    payload.timezone !== config.timezone ||
    payload.storageLayout !== config.storageLayout ||
    payload.installOpenSsh !== config.installOpenSsh ||
    payload.allowPasswordAuth !== config.allowPasswordAuth ||
    payload.authorizedKeys.join("\n") !== config.authorizedKeys.join("\n") ||
    payload.packages.join("\n") !== config.packages.join("\n")
  );
}

function validateForm(form: FormState, hasPassword: boolean): Record<string, string> {
  const errors: Record<string, string> = {};
  const hostname = form.hostname.trim();
  const username = form.username.trim();
  const password = form.password.trim();

  if (!hostname || hostname.length > 63 || hostname.startsWith("-") || hostname.endsWith("-")) {
    errors.hostname = "Hostname must be 1-63 characters and cannot start or end with a hyphen.";
  } else if (!/^[A-Za-z0-9-]+$/.test(hostname)) {
    errors.hostname = "Hostname may contain only letters, numbers, and hyphens.";
  }

  if (!/^[a-z_][a-z0-9_-]*$/.test(username)) {
    errors.username =
      "Username must start with a lowercase letter or underscore and use lowercase letters, numbers, underscores, or hyphens.";
  }

  if (!hasPassword && !password) {
    errors.password = "Password is required.";
  } else if (password && password.length < 8) {
    errors.password = "Password must be at least 8 characters.";
  }

  if (password !== form.confirmPassword.trim()) {
    errors.confirmPassword = "Password confirmation must match.";
  }

  if (!form.locale.trim()) {
    errors.locale = "Locale is required.";
  }
  if (!form.keyboardLayout.trim()) {
    errors.keyboardLayout = "Keyboard layout is required.";
  }
  if (!form.timezone.trim()) {
    errors.timezone = "Timezone is required.";
  }

  const invalidKey = linesToList(form.authorizedKeysText).find(
    (entry) =>
      !entry.startsWith("ssh-") &&
      !entry.startsWith("ecdsa-") &&
      !entry.startsWith("sk-"),
  );
  if (invalidKey) {
    errors.authorizedKeys = `Invalid SSH public key: ${invalidKey}`;
  }

  return errors;
}

function parseValidationError(error: unknown): ValidationErrorResponse {
  if (
    typeof error === "object" &&
    error !== null &&
    "data" in error &&
    typeof error.data === "object" &&
    error.data !== null &&
    "message" in error.data &&
    "fieldErrors" in error.data
  ) {
    const data = error.data as ValidationErrorResponse;
    return {
      message: data.message ?? "Failed to save autoinstall config.",
      fieldErrors: data.fieldErrors ?? {},
    };
  }

  return {
    message: "Failed to save autoinstall config.",
    fieldErrors: {},
  };
}
