import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  NativeSelect,
  Paper,
  PasswordInput,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Textarea,
  Title,
} from "@mantine/core";

import type {
  UbuntuAutoinstallConfig,
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
      <Paper component="section" p="xl">
        <Text size="lg">Loading autoinstall config...</Text>
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
    <Paper component="section" p="xl">
      <Stack gap="md">
        <div>
          <Text c="boopaAccent.7" fw={700} fz="xs" lts="0.16em" mb={6} tt="uppercase">
            Autoinstall
          </Text>
          <Title order={2} size="h3">
            Ubuntu autoinstall defaults
          </Title>
          <Text c="slate.7" mt="xs">
            Save the basic install options the backend will render into
            <Box component="span" fw={700}>
              {" "}
              `/boot/ubuntu/uefi/autoinstall/user-data`
            </Box>
            .
          </Text>
        </div>

        {serverMessage ? <Alert color="red">{serverMessage}</Alert> : null}

        <SimpleGrid cols={{ base: 1, xl: 2 }} spacing="lg">
          <Paper bg="rgba(244, 247, 249, 0.9)" p="lg" radius="28px">
            <Stack gap="md">
              <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md">
                <TextInput
                  label="Hostname"
                  value={form.hostname}
                  error={fieldErrors.hostname}
                  onChange={(event) => updateField("hostname", event.currentTarget.value)}
                />
                <TextInput
                  label="Username"
                  value={form.username}
                  error={fieldErrors.username}
                  onChange={(event) => updateField("username", event.currentTarget.value)}
                />
                <PasswordInput
                  label="Password"
                  value={form.password}
                  error={fieldErrors.password}
                  onChange={(event) => updateField("password", event.currentTarget.value)}
                  description={
                    query.data.hasPassword
                      ? "Leave blank to keep the saved password hash."
                      : "Required for the first save."
                  }
                />
                <PasswordInput
                  label="Confirm password"
                  value={form.confirmPassword}
                  error={fieldErrors.confirmPassword}
                  onChange={(event) => updateField("confirmPassword", event.currentTarget.value)}
                />
                <TextInput
                  label="Locale"
                  value={form.locale}
                  error={fieldErrors.locale}
                  onChange={(event) => updateField("locale", event.currentTarget.value)}
                />
                <TextInput
                  label="Keyboard layout"
                  value={form.keyboardLayout}
                  error={fieldErrors.keyboardLayout}
                  onChange={(event) => updateField("keyboardLayout", event.currentTarget.value)}
                />
                <TextInput
                  label="Timezone"
                  value={form.timezone}
                  error={fieldErrors.timezone}
                  onChange={(event) => updateField("timezone", event.currentTarget.value)}
                />
                <NativeSelect
                  label="Storage layout"
                  value={form.storageLayout}
                  data={[
                    { value: "direct", label: "Direct" },
                    { value: "lvm", label: "LVM" },
                  ]}
                  onChange={(event) =>
                    updateField("storageLayout", event.currentTarget.value as "direct" | "lvm")
                  }
                />
              </SimpleGrid>

              <Checkbox
                label="Install OpenSSH server"
                checked={form.installOpenSsh}
                onChange={(event) => updateField("installOpenSsh", event.currentTarget.checked)}
              />
              <Checkbox
                label="Allow password authentication"
                checked={form.allowPasswordAuth}
                onChange={(event) =>
                  updateField("allowPasswordAuth", event.currentTarget.checked)
                }
              />

              <Textarea
                label="Authorized keys"
                description="One SSH public key per line."
                minRows={4}
                value={form.authorizedKeysText}
                error={fieldErrors.authorizedKeys}
                onChange={(event) => updateField("authorizedKeysText", event.currentTarget.value)}
              />
              <Textarea
                label="Packages"
                description="One apt package name per line."
                minRows={4}
                value={form.packagesText}
                error={fieldErrors.packages}
                onChange={(event) => updateField("packagesText", event.currentTarget.value)}
              />

              <Button
                disabled={saveDisabled}
                loading={saveState.isLoading}
                variant="gradient"
                onClick={() => {
                  void handleSave();
                }}
              >
                Save Autoinstall Config
              </Button>
            </Stack>
          </Paper>

          <Paper bg="rgba(16, 24, 32, 0.96)" c="white" p="lg" radius="28px">
            <Stack gap="sm">
              <div>
                <Text c="boopaAccent.2" fw={700} fz="xs" lts="0.16em" tt="uppercase">
                  YAML Preview
                </Text>
                <Title order={3} size="h4" c="white" mt={6}>
                  Rendered user-data
                </Title>
              </div>
              <Text c="rgba(255,255,255,0.72)" size="sm">
                This preview reflects the last config saved to the backend.
              </Text>
              <Box
                component="pre"
                bg="rgba(255,255,255,0.04)"
                c="white"
                p="md"
                style={{
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
        </SimpleGrid>
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
