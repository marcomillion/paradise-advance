const { test, expect } = require('@playwright/test');

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
});

test('studio creates, renames, and restores a sprite after reload', async ({ page }) => {
  await page.locator('#newSpriteBtn').click();
  await expect(page.locator('#spriteCount')).toHaveText('1');

  await page.locator('#spriteName').fill('Regression Sprite');
  await page.locator('#spriteName').press('Tab');
  await expect(page.locator('#currentSpriteName')).toHaveText('REGRESSION SPRITE');

  await page.waitForFunction(() => {
    const saved = JSON.parse(localStorage.getItem('paradiseModStudio.workspace.v1') || '{}');
    return saved.sprites?.some(sprite => sprite.name === 'Regression Sprite');
  });

  await page.reload();
  await expect(page.locator('#spriteCount')).toHaveText('1');
  await expect(page.locator('#currentSpriteName')).toHaveText('REGRESSION SPRITE');
  await expect(page.locator('#spriteName')).toHaveValue('Regression Sprite');
});

test('sprite editor can paint a pixel and export a project JSON', async ({ page }) => {
  await page.locator('#newSpriteBtn').click();
  await page.locator('#colorPicker').evaluate(el => { el.value = '#ff0000'; el.dispatchEvent(new Event('input', { bubbles: true })); });
  await page.locator('#spriteCanvas').click({ position: { x: 5, y: 5 } });

  await page.waitForFunction(() => {
    const saved = JSON.parse(localStorage.getItem('paradiseModStudio.workspace.v1') || '{}');
    return saved.sprites?.length === 1 && saved.sprites[0].pixels[0] === '#ff0000';
  });

  const downloadPromise = page.waitForEvent('download');
  await page.locator('#exportJsonBtn').click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/\.json$/);
});

test('emulator status reports the browser core instead of showing a fake pet', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  await expect(page.locator('#emulatorStatusTitle')).toHaveText('WASM core ready · awaiting firmware', { timeout: 15_000 });
  await expect(page.locator('#emulatorRunCount')).toBeVisible();
  await expect(page.locator('#emulatorStepBtn')).toBeDisabled();
});
