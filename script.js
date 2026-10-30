const firmwareInput = document.getElementById("firmwareInput");
const firmwareStatus = document.getElementById("firmwareStatus");

const canvas = document.getElementById("spriteCanvas");
const ctx = canvas.getContext("2d");

const colorInput = document.getElementById("pixelColor");
const pencilButton = document.getElementById("pencilButton");
const eraserButton = document.getElementById("eraserButton");
const clearButton = document.getElementById("clearButton");
const exportButton = document.getElementById("exportButton");
const spriteLibrary = document.getElementById("spriteLibrary");

const WIDTH = 16;
const HEIGHT = 16;

let pixels = Array.from(
{ length: HEIGHT },
() => Array(WIDTH).fill(null)
);

let tool = "pencil";
let drawing = false;
let loadedFirmware = null;

function renderCanvas() {
ctx.clearRect(0, 0, WIDTH, HEIGHT);

for (let y = 0; y < HEIGHT; y++) {
    for (let x = 0; x < WIDTH; x++) {
        if (pixels[y][x]) {
            ctx.fillStyle = pixels[y][x];
            ctx.fillRect(x, y, 1, 1);
        }
    }
}

}

function setTool(nextTool) {
tool = nextTool;

pencilButton.classList.toggle("active", tool === "pencil");
eraserButton.classList.toggle("active", tool === "eraser");

}

function getPixelPosition(event) {
const rect = canvas.getBoundingClientRect();

const x = Math.floor(
    ((event.clientX - rect.left) / rect.width) * WIDTH
);

const y = Math.floor(
    ((event.clientY - rect.top) / rect.height) * HEIGHT
);

if (x < 0 || x >= WIDTH || y < 0 || y >= HEIGHT) {
    return null;
}

return { x, y };

}

function paintPixel(event) {
const position = getPixelPosition(event);

if (!position) return;

const { x, y } = position;

pixels[y][x] = tool === "eraser" ? null : colorInput.value;

renderCanvas();

}

canvas.addEventListener("pointerdown", (event) => {
drawing = true;
canvas.setPointerCapture(event.pointerId);
paintPixel(event);
});

canvas.addEventListener("pointermove", (event) => {
if (drawing) {
paintPixel(event);
}
});

function stopDrawing() {
drawing = false;
}

canvas.addEventListener("pointerup", stopDrawing);
canvas.addEventListener("pointercancel", stopDrawing);
canvas.addEventListener("lostpointercapture", stopDrawing);

pencilButton.addEventListener("click", () => {
setTool("pencil");
});

eraserButton.addEventListener("click", () => {
setTool("eraser");
});

clearButton.addEventListener("click", () => {
pixels = Array.from(
{ length: HEIGHT },
() => Array(WIDTH).fill(null)
);

renderCanvas();

});

firmwareInput.addEventListener("change", async () => {
const file = firmwareInput.files[0];

if (!file) return;

try {
    firmwareStatus.textContent = "Reading file locally…";
    firmwareStatus.classList.remove("success");

    // Read the file into browser memory. Nothing is uploaded.
    const buffer = await file.arrayBuffer();

    loadedFirmware = {
        name: file.name,
        size: buffer.byteLength,
        bytes: new Uint8Array(buffer)
    };

    const sizeText = loadedFirmware.size.toLocaleString();

    firmwareStatus.textContent =
        `Loaded: ${loadedFirmware.name} · ${sizeText} bytes. ` +
        "Firmware extraction is not implemented yet.";

    firmwareStatus.classList.add("success");
} catch (error) {
    loadedFirmware = null;
    firmwareStatus.textContent =
        "Unable to read this file. Please try another file.";
    console.error(error);
}

});

function createSpriteBlob() {
const exportCanvas = document.createElement("canvas");

exportCanvas.width = WIDTH;
exportCanvas.height = HEIGHT;

const exportCtx = exportCanvas.getContext("2d");

for (let y = 0; y < HEIGHT; y++) {
    for (let x = 0; x < WIDTH; x++) {
        if (pixels[y][x]) {
            exportCtx.fillStyle = pixels[y][x];
            exportCtx.fillRect(x, y, 1, 1);
        }
    }
}

return new Promise((resolve) => {
    exportCanvas.toBlob(resolve, "image/png");
});

}

exportButton.addEventListener("click", async () => {
const blob = await createSpriteBlob();

if (!blob) {
    alert("Could not export the sprite. Please try again.");
    return;
}

const downloadUrl = URL.createObjectURL(blob);
const link = document.createElement("a");

link.href = downloadUrl;
link.download = "paradise-sprite.png";
link.click();

setTimeout(() => URL.revokeObjectURL(downloadUrl), 1000);

addSpriteToLibrary(blob);

});

function addSpriteToLibrary(blob) {
if (spriteLibrary.querySelector(".empty-library")) {
spriteLibrary.innerHTML = "";
}

const card = document.createElement("div");
card.className = "sprite-card";

const preview = document.createElement("img");
const previewUrl = URL.createObjectURL(blob);

preview.src = previewUrl;
preview.alt = "Exported pixel sprite";
preview.width = 96;
preview.height = 96;
preview.style.imageRendering = "pixelated";
preview.style.background = "#252c3b";
preview.style.borderRadius = "6px";

const label = document.createElement("p");
label.textContent = "Custom sprite · 16 × 16";
label.className = "small-note";

card.append(preview, label);
spriteLibrary.prepend(card);

}

renderCanvas();
setTool("pencil");
