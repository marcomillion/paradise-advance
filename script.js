const firmwareInput = document.getElementById("firmwareInput");
const firmwareBadge = document.getElementById("firmwareBadge");
const firmwareDetails = document.getElementById("firmwareDetails");
const fileName = document.getElementById("fileName");

const canvases = [...document.querySelectorAll(".sprite-canvas")];
const simCanvas = document.getElementById("simCanvas");
const simContext = simCanvas.getContext("2d");

const colorInputs = [...document.querySelectorAll(".pixel-color")];
const toolButtons = [...document.querySelectorAll(".tool-button")];
const clearButtons = [...document.querySelectorAll(".clear-button")];
const exportButtons = [...document.querySelectorAll(".export-button")];

const WIDTH = 16;
const HEIGHT = 16;

let pixels = Array.from(
    { length: HEIGHT },
    () => Array(WIDTH).fill(null)
);

let selectedTool = "pencil";
let selectedColor = "#ff806e";
let loadedFirmware = null;
let activePointers = new Set();

// ----------------------------
// WORKSPACE NAVIGATION
// ----------------------------

function showView(name) {
    document.querySelectorAll(".mode-tab").forEach((tab) => {
        const selected = tab.dataset.view === name;
        tab.classList.toggle("active", selected);
        tab.setAttribute("aria-pressed", String(selected));
    });

    document.querySelectorAll(".view").forEach((view) => {
        view.classList.toggle("active", view.id === `view-${name}`);
    });
}

document.querySelectorAll(".mode-tab").forEach((tab) => {
    tab.addEventListener("click", () => showView(tab.dataset.view));
});

document.querySelectorAll("[data-go]").forEach((button) => {
    button.addEventListener("click", () => showView(button.dataset.go));
});

// ----------------------------
// PIXEL EDITOR
// ----------------------------

function renderCanvas(canvas) {
    const context = canvas.getContext("2d");

    context.clearRect(0, 0, WIDTH, HEIGHT);

    for (let y = 0; y < HEIGHT; y++) {
        for (let x = 0; x < WIDTH; x++) {
            if (pixels[y][x] !== null) {
                context.fillStyle = pixels[y][x];
                context.fillRect(x, y, 1, 1);
            }
        }
    }
}

function renderAll() {
    canvases.forEach(renderCanvas);
    renderCanvas(simCanvas);
}

function chooseTool(tool) {
    selectedTool = tool;

    toolButtons.forEach((button) => {
        button.classList.toggle(
            "active",
            button.dataset.tool === selectedTool
        );
    });
}

toolButtons.forEach((button) => {
    button.addEventListener("click", () => {
        chooseTool(button.dataset.tool);
    });
});

function chooseColor(color) {
    selectedColor = color;

    colorInputs.forEach((input) => {
        input.value = color;
    });
}

colorInputs.forEach((input) => {
    input.addEventListener("input", () => chooseColor(input.value));
});

document.querySelectorAll("[data-color]").forEach((swatch) => {
    swatch.addEventListener("click", () => {
        chooseColor(swatch.dataset.color);
        showView("lab");
    });
});

function paintAt(canvas, event) {
    const rect = canvas.getBoundingClientRect();

    const x = Math.floor(
        ((event.clientX - rect.left) / rect.width) * WIDTH
    );

    const y = Math.floor(
        ((event.clientY - rect.top) / rect.height) * HEIGHT
    );

    if (x < 0 || x >= WIDTH || y < 0 || y >= HEIGHT) {
        return;
    }

    pixels[y][x] = selectedTool === "eraser" ? null : selectedColor;

    renderAll();
}

canvases.forEach((canvas) => {
    canvas.addEventListener("pointerdown", (event) => {
        event.preventDefault();

        activePointers.add(event.pointerId);
        canvas.setPointerCapture(event.pointerId);
        paintAt(canvas, event);
    });

    canvas.addEventListener("pointermove", (event) => {
        if (activePointers.has(event.pointerId)) {
            event.preventDefault();
            paintAt(canvas, event);
        }
    });

    function stopDrawing(event) {
        activePointers.delete(event.pointerId);
    }

    canvas.addEventListener("pointerup", stopDrawing);
    canvas.addEventListener("pointercancel", stopDrawing);
    canvas.addEventListener("lostpointercapture", stopDrawing);
});

clearButtons.forEach((button) => {
    button.addEventListener("click", () => {
        const confirmed = window.confirm(
            "Clear the entire sprite? This cannot be undone."
        );

        if (!confirmed) return;

        pixels = Array.from(
            { length: HEIGHT },
            () => Array(WIDTH).fill(null)
        );

        renderAll();
    });
});

// ----------------------------
// PNG EXPORT
// ----------------------------

function createExportCanvas() {
    const output = document.createElement("canvas");

    output.width = WIDTH;
    output.height = HEIGHT;

    const context = output.getContext("2d");

    for (let y = 0; y < HEIGHT; y++) {
        for (let x = 0; x < WIDTH; x++) {
            if (pixels[y][x] !== null) {
                context.fillStyle = pixels[y][x];
                context.fillRect(x, y, 1, 1);
            }
        }
    }

    return output;
}

exportButtons.forEach((button) => {
    button.addEventListener("click", () => {
        const output = createExportCanvas();

        output.toBlob((blob) => {
            if (!blob) {
                alert("PNG export failed. Please try again.");
                return;
            }

            const url = URL.createObjectURL(blob);
            const link = document.createElement("a");

            link.href = url;
            link.download = "paradise-sprite.png";

            document.body.appendChild(link);
            link.click();
            link.remove();

            setTimeout(() => URL.revokeObjectURL(url), 1000);
        }, "image/png");
    });
});

// ----------------------------
// LOCAL FIRMWARE INSPECTOR
// ----------------------------

function formatBytes(bytes) {
    if (bytes < 1024) {
        return `${bytes.toLocaleString()} bytes`;
    }

    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(2)} KB`;
    }

    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

firmwareInput.addEventListener("change", async () => {
    const file = firmwareInput.files[0];

    if (!file) return;

    firmwareBadge.textContent = "● READING FILE";
    firmwareBadge.classList.remove("loaded");

    try {
        // Read the file locally. It is never uploaded to a server.
        const buffer = await file.arrayBuffer();

        loadedFirmware = {
            name: file.name,
            size: buffer.byteLength,
            bytes: new Uint8Array(buffer)
        };

        firmwareBadge.textContent = "● FILE LOADED";
        firmwareBadge.classList.add("loaded");

        fileName.textContent = file.name;

        firmwareDetails.replaceChildren();

        const icon = document.createElement("div");
        icon.className = "empty-symbol";
        icon.textContent = "✓";

        const info = document.createElement("div");

        const name = document.createElement("strong");
        name.textContent = loadedFirmware.name;

        const size = document.createElement("p");
        size.textContent = `File size: ${formatBytes(loadedFirmware.size)}`;

        const status = document.createElement("p");
        status.textContent =
            "File read successfully. Sprite extraction is not available yet.";

        info.append(name, size, status);
        firmwareDetails.append(icon, info);
    } catch (error) {
        loadedFirmware = null;

        firmwareBadge.textContent = "● READ FAILED";
        fileName.textContent = "No file selected";

        firmwareDetails.textContent =
            "The file could not be read. Try selecting it again.";

        console.error("Firmware read error:", error);
    }
});

// ----------------------------
// SIMULATOR PREVIEW
// ----------------------------

function updateSimulator() {
    renderCanvas(simCanvas);
}

window.addEventListener("resize", updateSimulator);

renderAll();
chooseTool("pencil");
chooseColor("#ff806e");
showView("studio");
