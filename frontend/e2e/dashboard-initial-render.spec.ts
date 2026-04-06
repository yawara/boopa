import { expect, test } from "@playwright/test";

test("renders the ubuntu dashboard against the live backend", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("img", { name: "boopa logo" })).toBeVisible();
  await expect(page.getByText("Asset readiness for ubuntu")).toBeVisible();
  await expect(page.getByText("Manual settings for ubuntu")).toBeVisible();
  await expect(page.getByText("Ubuntu autoinstall defaults")).toBeVisible();
});
