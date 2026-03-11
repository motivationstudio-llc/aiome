import { test, expect } from '@playwright/test';

test('has title', async ({ page }) => {
  await page.goto('/');

  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/Aiome Console/);
});

test('loads dashboard elements', async ({ page }) => {
  await page.goto('/');

  // Check if main UI structure exists
  // Adjust these locators depending on the actual layout, mostly just ensuring it doesn't blank page crash
  await expect(page.locator('#root')).toBeVisible();
});
