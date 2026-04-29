# Run And Debug WebGPU

## Run In A Browser

Build the wasm package from the project root:

```powershell
wasm-pack build --target web
```

Start a local HTTP server from the project root:

```powershell
python -m http.server 8000 --bind 127.0.0.1
```

Open this URL:

```text
http://127.0.0.1:8000/webTest/index.html
```

Serve the project root, not the `webTest` folder directly. `webTest/index.js` imports the generated wasm package from `../pkg/alEngine.js`, so both `webTest/` and `pkg/` must be available from the same server.

Do not open `webTest/index.html` with a `file://` URL. Browser module imports and wasm loading need an HTTP server.

## Development Loop

After changing Rust code:

```powershell
wasm-pack build --target web --dev
```

Then refresh the browser page.

After changing only files in `webTest/`, just refresh the browser.

The triangle shader is embedded into the wasm from:

```text
src/shaders/triangle.wgsl
```

If you edit this shader file, rebuild the wasm and refresh the browser:

```powershell
wasm-pack build --target web --dev
```

## Browser Debugging

Open DevTools with `F12`.

Use the Console tab first:

- Rust panics are reported through `console_error_panic_hook`.
- JavaScript errors from `webTest/index.js` appear there.
- WebGPU validation errors usually appear there too.

Use the Network tab and reload the page. You should see successful loads for:

- `/webTest/index.html`
- `/webTest/index.js`
- `/webTest/index.css`
- `/pkg/alEngine.js`
- `/pkg/alEngine_bg.wasm`

Use the Sources tab to inspect and debug `webTest/index.js` and the generated `pkg/alEngine.js`.

## Rust/Wasm Source Debugging

For better source-level wasm debugging in Chrome or Edge, install the browser extension:

```text
C/C++ DevTools Support (DWARF)
```

Then build without release optimizations:

```powershell
wasm-pack build --target web --dev
```

Reload the page with DevTools open.

## Common Issues

If the page says WebGPU is not available, use a recent Chrome or Edge version and make sure hardware acceleration is enabled.

If the page fails to import `../pkg/alEngine.js`, make sure the HTTP server was started from the project root.

If you get stale behavior after rebuilding, hard-refresh the page with `Ctrl+F5`.
