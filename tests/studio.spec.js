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

test('keyboard zoom-dial input emits press and release events', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  await page.evaluate(() => {
    window.__dialInputEvents = [];
    document.addEventListener('paradise:device-input', event => {
      const detail = event.detail || {};
      if (detail.source === 'keyboard' && detail.button === 'DIAL') {
        window.__dialInputEvents.push({ type: detail.type, pressed: detail.pressed });
      }
    });
  });
  await page.locator('#zoomDial').focus();
  await page.locator('#zoomDial').press('Enter');
  await expect.poll(() => page.evaluate(() => window.__dialInputEvents)).toEqual([
    { type: 'button', pressed: true },
    { type: 'button', pressed: false }
  ]);
});

test('local gallery imports, exports, and removes a sprite draft', async ({ page }) => {
  const project = {
    format: 'paradise-mod-studio-sprite',
    version: 1,
    name: 'Gallery Draft',
    width: 2,
    height: 2,
    palette: ['#ff0000', '#00ff00'],
    pixels: ['#ff0000', null, null, '#00ff00']
  };
  await page.locator('#galleryInput').setInputFiles({
    name: 'gallery-draft.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(project))
  });
  await expect(page.locator('#galleryGrid')).toContainText('Gallery Draft');
  await expect(page.locator('#galleryGrid').getByRole('button', { name: 'Export project JSON' })).toBeVisible();
  await page.locator('#galleryGrid').getByRole('button', { name: 'Remove draft' }).click();
  await expect(page.locator('#galleryGrid')).toContainText('The gallery is empty.');
});

test('emulator status reports the browser core instead of showing a fake pet', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  await expect(page.locator('#emulatorStatusTitle')).toHaveText('WASM core ready · awaiting firmware', { timeout: 15_000 });
  await expect(page.locator('#emulatorRunCount')).toBeVisible();
  await expect(page.locator('#emulatorStepBtn')).toBeDisabled();
});
