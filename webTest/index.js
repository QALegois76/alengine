import init, { run } from "../pkg/alEngine.js";

const status = document.querySelector("#status");

try {
    console.log("Hello alEngine");
    if (!("gpu" in navigator)) {
        throw new Error("WebGPU is not available in this browser.");
    }

    await init();
    await run();
} catch (error) {
    console.error(error);
    status.textContent = error?.message ?? String(error);
}
