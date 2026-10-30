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

test('emulator firmware button opens the local file picker', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  const chooserPromise = page.waitForEvent('filechooser');
  await page.locator('#emulatorLoadFirmwareBtn').click();
  const chooser = await chooserPromise;
  expect(chooser).toBeTruthy();
});

test('keyboard activation of a device button sends the correct input pulse', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  await page.evaluate(() => {
    window.__buttonInputEvents = [];
    document.addEventListener('paradise:device-input', event => {
      const detail = event.detail || {};
      if (detail.source === 'keyboard' && ['A', 'B', 'C'].includes(detail.button)) {
        window.__buttonInputEvents.push({ button: detail.button, pressed: detail.pressed });
      }
    });
  });
  await page.locator('[data-device-button="A"]').focus();
  await page.locator('[data-device-button="A"]').press('Enter');
  await expect.poll(() => page.evaluate(() => window.__buttonInputEvents)).toEqual([
    { button: 'A', pressed: true },
    { button: 'A', pressed: false }
  ]);
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

test('invalid workspace backup is rejected without replacing the current workspace', async ({ page }) => {
  await page.locator('#newSpriteBtn').click();
  await page.locator('#spriteName').fill('Keep This Sprite');
  await page.locator('#spriteName').press('Tab');

  const invalid = {
    format: 'paradise-mod-studio-workspace',
    version: 1,
    sprites: [{ id: 'bad', name: 'Oversized', width: 999, height: 999, pixels: [] }],
    gallery: [],
    activeId: 'bad'
  };
  await page.locator('#workspaceInput').setInputFiles({
    name: 'invalid-workspace.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(invalid))
  });
  await expect(page.locator('#toast')).toContainText('invalid dimensions');
  await expect(page.locator('#spriteCount')).toHaveText('1');
  await expect(page.locator('#currentSpriteName')).toHaveText('KEEP THIS SPRITE');
});

test('workspace backup restores sprites, gallery drafts, preferences, and animation', async ({ page }) => {
  const sprite = (id, name, color) => ({
    id, name, width: 2, height: 2,
    palette: [color],
    pixels: [color, null, null, color]
  });
  const backup = {
    format: 'paradise-mod-studio-workspace',
    version: 1,
    sprites: [sprite('backup-one', 'First Backup', '#ff0000'), sprite('backup-two', 'Second Backup', '#00ff00')],
    gallery: [sprite('backup-gallery', 'Gallery Backup', '#0000ff')],
    activeId: 'backup-two',
    color: '#00ff00',
    grid: false,
    zoom: 8,
    animation: { fps: 6, frames: [sprite('frame-one', 'Animation Frame', '#ff00ff')] }
  };
  await page.locator('#workspaceInput').setInputFiles({
    name: 'workspace-backup.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(backup))
  });
  await expect(page.locator('#spriteCount')).toHaveText('2');
  await expect(page.locator('#currentSpriteName')).toHaveText('SECOND BACKUP');
  await expect(page.locator('#galleryGrid')).toContainText('Gallery Backup');
  await expect(page.locator('#animationFrameCount')).toHaveText('1 frame');
  await expect(page.locator('#gridToggle')).not.toBeChecked();

  const downloadPromise = page.waitForEvent('download');
  await page.locator('#workspaceExportBtn').click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe('paradise-mod-studio-workspace.json');
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
  await page.locator('[data-view="gallery"]').click();
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
  await expect(page.locator('#keySearchTested')).toHaveText('—');
  await expect(page.locator('#keySearchRate')).toHaveText('—');
  await expect(page.locator('#keySearchPeakRate')).toHaveText('—');
  await expect(page.locator('#keySearchProgressTrack')).toHaveAttribute('aria-valuenow', '0.000000');
});

test('device key is remembered only after explicit opt-in', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  const keyInput = page.locator('#deviceKeyInput');
  const remember = page.locator('#rememberDeviceKey');

  await expect(remember).not.toBeChecked();
  await keyInput.fill('0x1234ABCD');
  await expect.poll(() => page.evaluate(() => localStorage.getItem('paradise-mod-studio-device-key-v1'))).toBeNull();

  await remember.check();
  await expect.poll(() => page.evaluate(() => localStorage.getItem('paradise-mod-studio-device-key-v1'))).toBe('0x1234ABCD');

  await page.reload();
  await page.locator('[data-view="emulator"]').click();
  await expect(page.locator('#deviceKeyInput')).toHaveValue('0x1234ABCD');
  await expect(page.locator('#rememberDeviceKey')).toBeChecked();
});

test('unchecking device-key persistence clears the saved key', async ({ page }) => {
  await page.locator('[data-view="emulator"]').click();
  await page.locator('#rememberDeviceKey').check();
  await page.locator('#deviceKeyInput').fill('ABCDEF12');
  await expect.poll(() => page.evaluate(() => localStorage.getItem('paradise-mod-studio-device-key-v1'))).toBe('0xABCDEF12');

  await page.locator('#rememberDeviceKey').uncheck();
  await expect.poll(() => page.evaluate(() => localStorage.getItem('paradise-mod-studio-device-key-v1'))).toBeNull();

  await page.reload();
  await page.locator('[data-view="emulator"]').click();
  await expect(page.locator('#rememberDeviceKey')).not.toBeChecked();
  await expect(page.locator('#deviceKeyInput')).toHaveValue('');
});

