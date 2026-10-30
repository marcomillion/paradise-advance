"use strict";

/* =========================================
   PARADISE ADVANCE
   Main website functionality
========================================= */

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];

/* =========================================
   ELEMENTS
========================================= */

const pixelCanvas = $("#pixel-canvas");
/* PIXEL LAB ZOOM */

let zoomLevel = 1;

const canvasStage = $(".canvas-stage");
const zoomControls = document.createElement("div");

zoomControls.className = "zoom-controls";

zoomControls.innerHTML = `
    <button type="button" id="zoom-out" title="Zoom out">−</button>

    <input
        type="range"
        id="zoom-slider"
        min="1"
        max="8"
        step="0.25"
        value="1"
        aria-label="Canvas zoom"
    >

    <span class="zoom-label" id="zoom-label">100%</span>

    <button type="button" id="zoom-in" title="Zoom in">+</button>

    <button type="button" id="zoom-reset" title="Reset zoom">
        Reset
    </button>
`;

canvasStage.before(zoomControls);

const zoomSlider = $("#zoom-slider");
const zoomLabel = $("#zoom-label");

function updateZoom() {
    zoomLevel = Number(zoomSlider.value);

    const baseSize = 512;

    pixelCanvas.style.width = `${baseSize * zoomLevel}px`;
    pixelCanvas.style.height = `${baseSize * zoomLevel}px`;

    zoomLabel.textContent = `${Math.round(zoomLevel * 100)}%`;

    updateCursorPreviewPosition();
}

$("#zoom-in").addEventListener("click", () => {
    zoomSlider.value = Math.min(
        8,
        Number(zoomSlider.value) + 0.25
    );

    updateZoom();
});

$("#zoom-out").addEventListener("click", () => {
    zoomSlider.value = Math.max(
        1,
        Number(zoomSlider.value) - 0.25
    );

    updateZoom();
});

$("#zoom-reset").addEventListener("click", () => {
    zoomSlider.value = 1;
    updateZoom();
});

zoomSlider.addEventListener("input", updateZoom);

updateZoom();
const pixelContext = pixelCanvas.getContext("2d", {
    willReadFrequently: true
});
/* PIXEL CURSOR PREVIEW */

const cursorPreview = document.createElement("div");

cursorPreview.className = "pixel-cursor-preview";

canvasStage.appendChild(cursorPreview);

function updateCursorPreviewPosition(event) {
    if (event) {
        const rect = pixelCanvas.getBoundingClientRect();

        if (
            event.clientX < rect.left ||
            event.clientX >= rect.right ||
            event.clientY < rect.top ||
            event.clientY >= rect.bottom
        ) {
            cursorPreview.style.display = "none";
            return;
        }

        const position = getCanvasPosition(event);

        const stageRect = canvasStage.getBoundingClientRect();

        const pixelWidth = rect.width / pixelCanvas.width;
        const pixelHeight = rect.height / pixelCanvas.height;

        const brushSize = currentTool === "fill"
            ? 1
            : Number(brushSizeSelect.value);

        cursorPreview.style.left =
            `${rect.left - stageRect.left + position.x * pixelWidth}px`;

        cursorPreview.style.top =
            `${rect.top - stageRect.top + position.y * pixelHeight}px`;

        cursorPreview.style.width =
            `${Math.min(brushSize, pixelCanvas.width - position.x) * pixelWidth}px`;

        cursorPreview.style.height =
            `${Math.min(brushSize, pixelCanvas.height - position.y) * pixelHeight}px`;

        cursorPreview.style.display = "block";

        return;
    }

    cursorPreview.style.display = "none";
}

pixelCanvas.addEventListener("pointermove", (event) => {
    updateCursorPreviewPosition(event);
});

pixelCanvas.addEventListener("pointerleave", () => {
    updateCursorPreviewPosition();
});

const simulatorCanvas = $("#simulator-screen");
const simulatorContext = simulatorCanvas.getContext("2d");

const spriteCanvas = $("#sprite-canvas");
const spriteContext = spriteCanvas.getContext("2d", {
    willReadFrequently: true
});

const canvasSizeSelect = $("#canvas-size");
const brushSizeSelect = $("#brush-size");
const colorPicker = $("#color-picker");
const colorHex = $("#color-hex");

let currentTool = "pencil";
let currentColor = "#8b9cff";
let isDrawing = false;

let undoStack = [];
let redoStack = [];

let importedSprite = null;
let importedSpriteName = "";

let firmwareFile = null;

let toastTimer = null;

/* =========================================
   HELPERS
========================================= */

function showToast(message) {
    const toast = $("#toast");

    toast.textContent = message;
    toast.classList.add("visible");

    clearTimeout(toastTimer);

    toastTimer = setTimeout(() => {
        toast.classList.remove("visible");
    }, 2600);
}

function formatBytes(bytes) {
    if (bytes < 1024) {
        return `${bytes} bytes`;
    }

    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(2)} KB`;
    }

    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function validHex(value) {
    return /^#[0-9a-fA-F]{6}$/.test(value);
}

function updateCanvasInfo() {
    $("#canvas-info").textContent =
        `${pixelCanvas.width} × ${pixelCanvas.height} pixels`;
}

function updateCursorInfo(message) {
    $("#cursor-info").textContent = message;
}

/* =========================================
   NAVIGATION
========================================= */

$$(".nav-button").forEach((button) => {
    button.addEventListener("click", () => {
        const viewId = button.dataset.view;

        $$(".nav-button").forEach((item) => {
            item.classList.toggle("active", item === button);
        });

        $$(".view").forEach((view) => {
            view.classList.toggle("active", view.id === viewId);
        });
    });
});

/* =========================================
   CANVAS HISTORY
========================================= */

function saveUndoState() {
    try {
        const snapshot = pixelContext.getImageData(
            0,
            0,
            pixelCanvas.width,
            pixelCanvas.height
        );

        undoStack.push({
            width: pixelCanvas.width,
            height: pixelCanvas.height,
            image: snapshot
        });

        // Prevent unlimited memory use.
        if (undoStack.length > 30) {
            undoStack.shift();
        }

        redoStack = [];
    } catch (error) {
        console.error("Could not save undo state:", error);
    }
}

function restoreState(state) {
    pixelCanvas.width = state.width;
    pixelCanvas.height = state.height;

    pixelContext.putImageData(state.image, 0, 0);

    canvasSizeSelect.value = [16, 32, 64, 128, 256, 512]
        .includes(state.width) && state.width === state.height
        ? String(state.width)
        : "";

    updateCanvasInfo();
    updateSimulator();

    updateCursorInfo("Canvas restored");
}

function undo() {
    if (undoStack.length <= 1) {
        showToast("Nothing to undo.");
        return;
    }

    const current = undoStack.pop();
    redoStack.push(current);

    restoreState(undoStack[undoStack.length - 1]);

    showToast("Undo complete.");
}

function redo() {
    if (redoStack.length === 0) {
        showToast("Nothing to redo.");
        return;
    }

    const state = redoStack.pop();

    undoStack.push(state);
    restoreState(state);

    showToast("Redo complete.");
}

$("#undo-button").addEventListener("click", undo);
$("#redo-button").addEventListener("click", redo);

/* =========================================
   CANVAS RESIZING
========================================= */

function resizeCanvas(size, preserve = false) {
    size = Number(size);

    if (!Number.isInteger(size) || size < 1 || size > 2048) {
        showToast("Invalid canvas size.");
        return;
    }

    let oldCanvas = null;

    if (preserve) {
        oldCanvas = document.createElement("canvas");
        oldCanvas.width = pixelCanvas.width;
        oldCanvas.height = pixelCanvas.height;

        oldCanvas.getContext("2d").drawImage(pixelCanvas, 0, 0);
    }

    pixelCanvas.width = size;
    pixelCanvas.height = size;

    pixelContext.clearRect(0, 0, size, size);
    pixelContext.imageSmoothingEnabled = false;

    if (oldCanvas) {
        pixelContext.drawImage(
            oldCanvas,
            0,
            0,
            oldCanvas.width,
            oldCanvas.height,
            0,
            0,
            size,
            size
        );
    }

    undoStack = [];
    redoStack = [];

    saveUndoState();

    updateCanvasInfo();
    updateSimulator();
    updateZoom();

    updateCursorInfo("Canvas resized");
}

canvasSizeSelect.addEventListener("change", () => {
    resizeCanvas(canvasSizeSelect.value);
});

/* =========================================
   DRAWING TOOLS
========================================= */

function selectTool(tool) {
    currentTool = tool;

    $$(".tool-button").forEach((button) => {
        button.classList.toggle(
            "selected",
            button.dataset.tool === tool
        );
    });

    pixelCanvas.style.cursor =
        tool === "eraser" ? "cell" : "crosshair";

    updateCursorInfo(`Tool: ${tool}`);
}

$$(".tool-button").forEach((button) => {
    button.addEventListener("click", () => {
        selectTool(button.dataset.tool);
    });
});

function getCanvasPosition(event) {
    const rect = pixelCanvas.getBoundingClientRect();

    const x = Math.floor(
        (event.clientX - rect.left) *
        (pixelCanvas.width / rect.width)
    );

    const y = Math.floor(
        (event.clientY - rect.top) *
        (pixelCanvas.height / rect.height)
    );

    return {
        x: Math.max(0, Math.min(pixelCanvas.width - 1, x)),
        y: Math.max(0, Math.min(pixelCanvas.height - 1, y))
    };
}

function drawAt(x, y) {
    const brushSize = Number(brushSizeSelect.value);

    if (currentTool === "eraser") {
        pixelContext.clearRect(
            x,
            y,
            brushSize,
            brushSize
        );

        return;
    }

    pixelContext.fillStyle = currentColor;

    pixelContext.fillRect(
        x,
        y,
        brushSize,
        brushSize
    );
}

function startDrawing(event) {
    if (event.button !== undefined && event.button !== 0) {
        return;
    }

    event.preventDefault();

    const position = getCanvasPosition(event);
    updateCursorPreviewPosition(event);

    if (currentTool === "fill") {
        saveUndoState();

        floodFill(position.x, position.y, currentColor);

        updateSimulator();

        return;
    }

    saveUndoState();

    isDrawing = true;

    pixelCanvas.setPointerCapture?.(event.pointerId);

    drawAt(position.x, position.y);

    updateCursorInfo(
        `X: ${position.x}, Y: ${position.y}`
    );

    updateSimulator();
}

function continueDrawing(event) {
    if (!isDrawing) {
        return;
    }

    event.preventDefault();

    const position = getCanvasPosition(event);
   updateCursorPreviewPosition(event);

    drawAt(position.x, position.y);

    updateCursorInfo(
        `X: ${position.x}, Y: ${position.y}`
    );

    updateSimulator();
}

function stopDrawing() {
    isDrawing = false;
}

pixelCanvas.addEventListener("pointerdown", startDrawing);
pixelCanvas.addEventListener("pointermove", continueDrawing);
pixelCanvas.addEventListener("pointerup", stopDrawing);
pixelCanvas.addEventListener("pointercancel", stopDrawing);
pixelCanvas.addEventListener("lostpointercapture", stopDrawing);

window.addEventListener("pointerup", stopDrawing);

/* =========================================
   FLOOD FILL
========================================= */

function floodFill(startX, startY, fillColor) {
    const width = pixelCanvas.width;
    const height = pixelCanvas.height;

    const imageData = pixelContext.getImageData(
        0,
        0,
        width,
        height
    );

    const data = imageData.data;

    const startIndex = (startY * width + startX) * 4;

    const target = [
        data[startIndex],
        data[startIndex + 1],
        data[startIndex + 2],
        data[startIndex + 3]
    ];

    const hex = fillColor.replace("#", "");

    const replacement = [
        parseInt(hex.slice(0, 2), 16),
        parseInt(hex.slice(2, 4), 16),
        parseInt(hex.slice(4, 6), 16),
        255
    ];

    if (
        target[0] === replacement[0] &&
        target[1] === replacement[1] &&
        target[2] === replacement[2] &&
        target[3] === replacement[3]
    ) {
        return;
    }

    function matches(index) {
        return (
            data[index] === target[0] &&
            data[index + 1] === target[1] &&
            data[index + 2] === target[2] &&
            data[index + 3] === target[3]
        );
    }

    const stack = [[startX, startY]];
    let processed = 0;

    while (stack.length > 0) {
        const [x, y] = stack.pop();

        if (
            x < 0 ||
            y < 0 ||
            x >= width ||
            y >= height
        ) {
            continue;
        }

        const index = (y * width + x) * 4;

        if (!matches(index)) {
            continue;
        }

        data[index] = replacement[0];
        data[index + 1] = replacement[1];
        data[index + 2] = replacement[2];
        data[index + 3] = replacement[3];

        processed++;

        if (processed > width * height) {
            break;
        }

        stack.push(
            [x + 1, y],
            [x - 1, y],
            [x, y + 1],
            [x, y - 1]
        );
    }

    pixelContext.putImageData(imageData, 0, 0);
}

/* =========================================
   COLOUR PICKER
========================================= */

colorPicker.addEventListener("input", () => {
    currentColor = colorPicker.value.toLowerCase();

    colorHex.value = currentColor.toUpperCase();
});

colorHex.addEventListener("change", () => {
    const value = colorHex.value.trim();

    if (!validHex(value)) {
        colorHex.value = currentColor.toUpperCase();

        showToast("Enter a valid hex colour, such as #8B9CFF.");
        return;
    }

    currentColor = value.toLowerCase();
    colorPicker.value = currentColor;
});

$("#palette").addEventListener("click", (event) => {
    const button = event.target.closest("[data-color]");

    if (!button) {
        return;
    }

    currentColor = button.dataset.color.toLowerCase();

    colorPicker.value = currentColor;
    colorHex.value = currentColor.toUpperCase();
});

/* =========================================
   CLEAR CANVAS
========================================= */

$("#clear-canvas").addEventListener("click", () => {
    saveUndoState();

    pixelContext.clearRect(
        0,
        0,
        pixelCanvas.width,
        pixelCanvas.height
    );

    updateSimulator();
    updateCursorInfo("Canvas cleared");

    showToast("Canvas cleared.");
});

/* =========================================
   IMAGE IMPORT
========================================= */

function loadImageFile(file, callback) {
    if (!file || !file.type.startsWith("image/")) {
        showToast("Please choose a valid image.");
        return;
    }

    const url = URL.createObjectURL(file);
    const image = new Image();

    image.onload = () => {
        callback(image, file.name);

        URL.revokeObjectURL(url);
    };

    image.onerror = () => {
        URL.revokeObjectURL(url);
        showToast("Could not open this image.");
    };

    image.src = url;
}

$("#image-input").addEventListener("change", (event) => {
    const file = event.target.files[0];

    if (!file) {
        return;
    }

    loadImageFile(file, (image, name) => {
        if (image.width > 2048 || image.height > 2048) {
            showToast("Images larger than 2048 pixels per side are not supported.");
            return;
        }

        pixelCanvas.width = image.width;
        pixelCanvas.height = image.height;

        pixelContext.clearRect(
            0,
            0,
            image.width,
            image.height
        );

        pixelContext.imageSmoothingEnabled = false;

        pixelContext.drawImage(
            image,
            0,
            0,
            image.width,
            image.height
        );

        undoStack = [];
        redoStack = [];

        saveUndoState();

        canvasSizeSelect.value =
            image.width === image.height &&
            [16, 32, 64, 128, 256, 512].includes(image.width)
                ? String(image.width)
                : "";

        updateCanvasInfo();
        updateSimulator();

        updateCursorInfo(`Imported: ${name}`);

        showToast(
            `Imported ${name} (${image.width} × ${image.height}).`
        );
    });

    event.target.value = "";
});

/* =========================================
   EXPORT PNG
========================================= */

$("#export-png").addEventListener("click", () => {
    pixelCanvas.toBlob((blob) => {
        if (!blob) {
            showToast("PNG export failed.");
            return;
        }

        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");

        link.href = url;
        link.download =
            `paradise-sprite-${pixelCanvas.width}x${pixelCanvas.height}.png`;

        link.click();

        setTimeout(() => URL.revokeObjectURL(url), 1000);

        showToast("PNG exported.");
    }, "image/png");
});

/* =========================================
   LOCAL CREATION STORAGE
========================================= */

const STORAGE_KEY = "paradiseAdvanceCreations";

function getSavedCreations() {
    try {
        return JSON.parse(
            localStorage.getItem(STORAGE_KEY) || "[]"
        );
    } catch (error) {
        console.error(error);
        return [];
    }
}

function saveCreation() {
    try {
        const creations = getSavedCreations();

        const png = pixelCanvas.toDataURL("image/png");

        creations.unshift({
            id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
            name: `Sprite ${pixelCanvas.width}x${pixelCanvas.height}`,
            width: pixelCanvas.width,
            height: pixelCanvas.height,
            png
        });

        // Keep browser storage usage under control.
        const limitedCreations = creations.slice(0, 15);

        localStorage.setItem(
            STORAGE_KEY,
            JSON.stringify(limitedCreations)
        );

        renderSavedCreations();

        showToast("Creation saved in this browser.");
    } catch (error) {
        console.error(error);

        showToast(
            "Could not save. Browser storage may be full."
        );
    }
}

$("#save-creation").addEventListener("click", saveCreation);

function renderSavedCreations() {
    const container = $("#saved-creations");
    const creations = getSavedCreations();

    container.replaceChildren();

    if (creations.length === 0) {
        const message = document.createElement("p");

        message.className = "muted";
        message.textContent = "No saved creations yet.";

        container.appendChild(message);
        return;
    }

    creations.forEach((creation) => {
        const card = document.createElement("div");
        card.className = "saved-card";

        const preview = document.createElement("canvas");
        preview.width = creation.width;
        preview.height = creation.height;

        const previewContext = preview.getContext("2d");
        const image = new Image();

        image.onload = () => {
            previewContext.clearRect(
                0,
                0,
                preview.width,
                preview.height
            );

            previewContext.imageSmoothingEnabled = false;
            previewContext.drawImage(image, 0, 0);
        };

        image.src = creation.png;

        const name = document.createElement("p");
        name.textContent = creation.name;

        const loadButton = document.createElement("button");
        loadButton.className = "button";
        loadButton.textContent = "Load";

        loadButton.addEventListener("click", () => {
            image.onload = () => {
                pixelCanvas.width = creation.width;
                pixelCanvas.height = creation.height;

                pixelContext.clearRect(
                    0,
                    0,
                    creation.width,
                    creation.height
                );

                pixelContext.drawImage(image, 0, 0);

                undoStack = [];
                redoStack = [];

                saveUndoState();

                canvasSizeSelect.value =
                    [16, 32, 64, 128, 256, 512].includes(creation.width) &&
                    creation.width === creation.height
                        ? String(creation.width)
                        : "";

                updateCanvasInfo();
                updateSimulator();

                showToast("Creation loaded.");
            };

            image.src = creation.png;
        });

        const deleteButton = document.createElement("button");
        deleteButton.className = "button";
        deleteButton.textContent = "Delete";

        deleteButton.addEventListener("click", () => {
            const remaining = getSavedCreations().filter(
                (item) => item.id !== creation.id
            );

            localStorage.setItem(
                STORAGE_KEY,
                JSON.stringify(remaining)
            );

            renderSavedCreations();

            showToast("Saved creation deleted.");
        });

        card.append(preview, name, loadButton, deleteButton);
        container.appendChild(card);
    });
}

/* =========================================
   PARADISE SIMULATOR
========================================= */

function updateSimulator() {
    simulatorContext.clearRect(
        0,
        0,
        simulatorCanvas.width,
        simulatorCanvas.height
    );

    // Simulate the pale green display background.
    simulatorContext.fillStyle = "#b9d5a2";

    simulatorContext.fillRect(
        0,
        0,
        simulatorCanvas.width,
        simulatorCanvas.height
    );

    simulatorContext.imageSmoothingEnabled = false;

    const scale = Math.min(
        simulatorCanvas.width / pixelCanvas.width,
        simulatorCanvas.height / pixelCanvas.height
    );

    const drawWidth = pixelCanvas.width * scale;
    const drawHeight = pixelCanvas.height * scale;

    const offsetX = (simulatorCanvas.width - drawWidth) / 2;
    const offsetY = (simulatorCanvas.height - drawHeight) / 2;

    simulatorContext.drawImage(
        pixelCanvas,
        0,
        0,
        pixelCanvas.width,
        pixelCanvas.height,
        offsetX,
        offsetY,
        drawWidth,
        drawHeight
    );

    $("#simulator-source").textContent =
        `${pixelCanvas.width} × ${pixelCanvas.height} artwork`;
}

$("#send-to-simulator").addEventListener("click", () => {
    updateSimulator();

    $$(".nav-button").forEach((button) => {
        button.classList.toggle(
            "active",
            button.dataset.view === "simulator"
        );
    });

    $$(".view").forEach((view) => {
        view.classList.toggle(
            "active",
            view.id === "simulator"
        );
    });

    showToast("Artwork sent to the simulator preview.");
});

$("#refresh-simulator").addEventListener("click", () => {
    updateSimulator();

    showToast("Simulator preview updated.");
});

$$(".device-button").forEach((button) => {
    button.addEventListener("click", () => {
        const input = button.dataset.input;

        $("#button-feedback").textContent =
            `Virtual button ${input} pressed.`;

        showToast(`Virtual button ${input} pressed.`);
    });
});

/* =========================================
   FIRMWARE STUDIO
========================================= */

function detectFormat(filename) {
    const extension = filename.split(".").pop().toLowerCase();

    const formats = {
        bin: "Binary (.bin)",
        hex: "Intel HEX (.hex)",
        uf2: "UF2 (.uf2)",
        img: "Disk image (.img)",
        rom: "ROM (.rom)",
        fw: "Firmware (.fw)",
        dat: "Data file (.dat)",
        elf: "ELF executable (.elf)"
    };

    return formats[extension] || "Unknown / unrecognized extension";
}

$("#firmware-input").addEventListener("change", async (event) => {
    const file = event.target.files[0];

    if (!file) {
        return;
    }

    firmwareFile = file;

    $("#firmware-empty").hidden = true;
    $("#firmware-details").hidden = false;

    $("#firmware-name").textContent = file.name;
    $("#firmware-size").textContent = formatBytes(file.size);
    $("#firmware-format").textContent = detectFormat(file.name);

    try {
        // Read only the first 32 bytes for a basic header preview.
        const header = await file.slice(0, 32).arrayBuffer();

        const bytes = new Uint8Array(header);

        const hex = [...bytes]
            .map((byte) => byte.toString(16).padStart(2, "0"))
            .join(" ")
            .toUpperCase();

        $("#firmware-magic").textContent =
            hex || "(empty file)";

        showToast("Firmware file opened locally.");
    } catch (error) {
        console.error(error);

        $("#firmware-magic").textContent =
            "Could not read file header.";

        showToast("Could not inspect this file.");
    }

    event.target.value = "";
});

/* =========================================
   SPRITE VIEWER
========================================= */

$("#sprite-input").addEventListener("change", (event) => {
    const file = event.target.files[0];

    if (!file) {
        return;
    }

    loadImageFile(file, (image, name) => {
        importedSprite = image;
        importedSpriteName = name;

        spriteCanvas.width = image.width;
        spriteCanvas.height = image.height;

        spriteContext.clearRect(
            0,
            0,
            spriteCanvas.width,
            spriteCanvas.height
        );

        spriteContext.imageSmoothingEnabled = false;

        spriteContext.drawImage(
            image,
            0,
            0,
            image.width,
            image.height
        );

        $("#sprite-name").textContent = name;

        $("#sprite-dimensions").textContent =
            `Original dimensions: ${image.width} × ${image.height}`;

        showToast("Image loaded in Sprite Viewer.");
    });

    event.target.value = "";
});

$("#edit-sprite").addEventListener("click", () => {
    if (!importedSprite) {
        showToast("Open an image in Sprite Viewer first.");
        return;
    }

    pixelCanvas.width = importedSprite.width;
    pixelCanvas.height = importedSprite.height;

    pixelContext.clearRect(
        0,
        0,
        pixelCanvas.width,
        pixelCanvas.height
    );

    pixelContext.imageSmoothingEnabled = false;

    pixelContext.drawImage(
        importedSprite,
        0,
        0
    );

    undoStack = [];
    redoStack = [];

    saveUndoState();

    canvasSizeSelect.value =
        importedSprite.width === importedSprite.height &&
        [16, 32, 64, 128, 256, 512].includes(importedSprite.width)
            ? String(importedSprite.width)
            : "";

    updateCanvasInfo();
    updateSimulator();

    $$(".nav-button").forEach((button) => {
        button.classList.toggle(
            "active",
            button.dataset.view === "pixel-lab"
        );
    });

    $$(".view").forEach((view) => {
        view.classList.toggle(
            "active",
            view.id === "pixel-lab"
        );
    });

    updateCursorInfo(`Editing: ${importedSpriteName}`);

    showToast("Sprite opened in Pixel Lab.");
});

/* =========================================
   INITIALIZATION
========================================= */

function initialize() {
    pixelContext.clearRect(
        0,
        0,
        pixelCanvas.width,
        pixelCanvas.height
    );

    pixelContext.imageSmoothingEnabled = false;
    spriteContext.imageSmoothingEnabled = false;
    simulatorContext.imageSmoothingEnabled = false;

    saveUndoState();

    updateCanvasInfo();
    updateSimulator();
    renderSavedCreations();

    console.log("Paradise Advance initialized.");
}

initialize();
