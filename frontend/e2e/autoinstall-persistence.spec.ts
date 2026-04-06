import { expect, test } from "@playwright/test";

test("persists autoinstall updates after reload", async ({ page }) => {
  const hostname = `e2e-node-${Date.now().toString().slice(-8)}`;

  await page.goto("/");

  const hostnameField = page.getByLabel("Hostname");
  await expect(hostnameField).toBeVisible();

  await hostnameField.fill(hostname);
  await page.getByRole("button", { name: "Save Autoinstall Config" }).click();

  await expect(page.getByText(new RegExp(`hostname:\\s+${hostname}`))).toBeVisible();

  await page.reload();

  await expect(page.getByLabel("Hostname")).toHaveValue(hostname);
  await expect(page.getByText(new RegExp(`hostname:\\s+${hostname}`))).toBeVisible();
});
