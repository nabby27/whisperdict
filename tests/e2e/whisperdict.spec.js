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
    await expect(page.getByTestId("model-small"), "model card ready").toContainText("Active", {
      timeout: 15_000,
    });
  });

  test("deletes an installed model", async ({ page }) => {
    await page.getByTestId("model-small-download").click();
    await expect(page.getByTestId("model-small"), "model card ready").toContainText("Active", {
      timeout: 15_000,
    });
    page.once("dialog", (dialog) => dialog.accept());
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
    await expect(page.getByTestId("status-label")).toContainText("Recording");
    await page.getByTestId("dictate-toggle").click();
    await expect(page.getByTestId("status-label")).toContainText("Transcribing");
    await expect(page.getByTestId("test-textarea")).toContainText("Mock transcription", {
      timeout: 10_000,
    });
  });

  test("shows upgrade modal when free limit is reached", async ({ page }) => {
    await page.evaluate(() => {
      window.__WHISPERDICT_MOCK__?.setFreeTranscriptionsLeft(0);
    });

    await page.getByTestId("dictate-toggle").click();

    await expect(page.getByTestId("free-limit-modal")).toBeVisible();
    await expect(page.getByTestId("free-limit-modal")).toContainText("Free plan limit reached");
    await expect(page.getByTestId("free-limit-get-pro")).toBeVisible();
    await expect(page.getByTestId("free-limit-import")).toBeVisible();
  });

  test("imports invalid license and keeps app locked", async ({ page }) => {
    await page.evaluate(() => {
      window.__WHISPERDICT_MOCK__?.setNextLicensePath("/tmp/invalid-license.json");
    });
    await page.getByTestId("import-license-button").click();

    await expect(page.getByTestId("status-message")).toContainText(/license file is invalid/i);
  });

  test("imports valid license and activates pro", async ({ page }) => {
    await page.evaluate(() => {
      window.__WHISPERDICT_MOCK__?.setNextLicensePath("/tmp/customer-license.json");
    });
    await page.getByTestId("import-license-button").click();

    await expect(page.getByTestId("status-message")).toHaveCount(0);
  });

  test("get pro button opens checkout flow", async ({ page }) => {
    await page.evaluate(() => {
      window.__openedUrl = null;
      window.open = (url) => {
        window.__openedUrl = String(url);
        return null;
      };
    });

    await page.getByTestId("get-pro-button").click();

    await expect
      .poll(async () => page.evaluate(() => window.__openedUrl))
      .toContain("polar.sh/checkout/mock-whisperdict");
  });
});
