import { test, expect } from "@playwright/test";

test.describe("Whisperdict app", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("renders model catalog", async ({ page }) => {
    await expect(page.getByTestId("model-tiny")).toBeVisible();
    await expect(page.getByTestId("model-base")).toBeVisible();
    await expect(page.getByTestId("model-small")).toBeVisible();
    await expect(page.getByTestId("model-medium")).toBeVisible();
    await expect(page.getByTestId("model-large")).toBeVisible();
  });

  test("downloads and activates a model", async ({ page }) => {
    const downloadButton = page.getByTestId("model-small-download");
    await downloadButton.click();
    await expect(page.getByTestId("model-small-progress")).toBeVisible();
    await expect(page.getByTestId("model-small"), "model card ready").toContainText("En uso", {
      timeout: 15_000,
    });
  });

  test("deletes an installed model", async ({ page }) => {
    await page.getByTestId("model-small-download").click();
    await expect(page.getByTestId("model-small"), "model card ready").toContainText("En uso", {
      timeout: 15_000,
    });
    await page.getByTestId("model-small-delete").click();
    await expect(page.getByTestId("model-small-download")).toBeVisible();
  });

  test("updates shortcut", async ({ page }) => {
    await page.getByTestId("shortcut-input").fill("Ctrl+Alt+D");
    await page.getByTestId("shortcut-save").click();
    await expect(page.getByTestId("active-shortcut")).toContainText("Ctrl+Alt+D");
  });

  test("dictation cycle writes into test area", async ({ page }) => {
    await page.getByTestId("dictate-toggle").click();
    await expect(page.getByTestId("status-label")).toContainText("Grabando");
    await page.getByTestId("dictate-toggle").click();
    await expect(page.getByTestId("status-label")).toContainText("Transcribiendo");
    await expect(page.getByTestId("test-textarea")).toContainText("Transcripcion", {
      timeout: 10_000,
    });
  });
});
