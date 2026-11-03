# Hosting WordCraft for the web

`wordcraft-web-<version>.zip` (from the GitHub release, or `packaging/web/package.sh`) holds a
static site in `wordcraft-web-<version>/`:

| File | What it is |
|---|---|
| `index.html` | The page. It loads everything through relative URLs. |
| `wordcraft-web-<hash>.js` | wasm-bindgen glue (generated, ES module) |
| `wordcraft-web-<hash>_bg.wasm` | The app, about 24 MB, or 10 MB with gzip and 7 MB with Brotli |
| `_headers`, `.htaccess` | Sample header rules for Netlify/Cloudflare Pages and Apache |

There is no server-side code. Upload the folder's contents anywhere that serves static files.

## Any path works

All URLs in `index.html` are relative (`public_url = "./"` in `apps/wordcraft-web/Trunk.toml`),
so the site works at a domain root (`https://example.com/`), under a prefix
(`https://example.com/tools/wordcraft/`) and from a CDN bucket. The asset names carry a content
hash, so they can be cached forever. Only `index.html` needs revalidation.

## Required server settings

- **MIME type:** serve `.wasm` as `application/wasm`. Browsers refuse to stream-compile it under
  any other type, and the app then loads slowly or not at all. Serve `.js` as `text/javascript`.
  Most hosts already do both. For nginx, check that `mime.types` has `application/wasm wasm;`.
- **Compression:** turn on gzip or Brotli for `.wasm`, `.js` and `.html`. That takes the
  download from about 24 MB to about 10 MB (gzip) or 7 MB (Brotli). You can also precompress (`brotli -k *.wasm`) and let
  the server send `Content-Encoding: br`.
- **Caching:** `Cache-Control: public, max-age=31536000, immutable` on the hashed `.wasm` and
  `.js` files, and `no-cache` on `index.html`.
- **HTTPS:** WebGPU (and the clipboard) only work in a secure context, which means `https://`
  or `http://localhost`. Over plain HTTP elsewhere, the app falls back to WebGL2.
- **No special isolation headers:** WordCraft doesn't use `SharedArrayBuffer`, so it doesn't
  need `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy`. If your site already sends
  COEP `require-corp`, also send `Cross-Origin-Resource-Policy: same-origin` (or `cross-origin`
  when the files live on a CDN) on the app's files.

nginx example:

```nginx
location /wordcraft/ {
    types { application/wasm wasm; text/javascript js; text/html html; }
    gzip on;
    gzip_types application/wasm text/javascript text/html;
    location ~* \.(wasm|js)$ { add_header Cache-Control "public, max-age=31536000, immutable"; }
    location ~* index\.html$ { add_header Cache-Control "no-cache"; }
}
```

Local test: `python3 -m http.server 8765` inside the folder, then open http://localhost:8765/.

## Download size

Build the site with the `web` Cargo profile: `trunk build --release --cargo-profile web` in
`apps/wordcraft-web` (`packaging/web/package.sh` does this). The profile, `[profile.web]` in the
workspace `Cargo.toml`, inherits `release` and builds for size: `opt-level = "z"`, fat LTO, one
codegen unit, `strip = true` and `panic = "abort"`. Only builds that name it use it; `cargo build
--release` and the desktop app are unchanged. Trunk then runs `wasm-opt -Oz`
(`data-wasm-opt="z"` in `index.html`).

Sizes of the `_bg.wasm`, measured without the optional `CRAFT_FONTS_DIR` and `CRAFT_I18N_DIR`
inputs (each embeds more data when set). MB here is MiB:

| Build | Raw | gzip -9 | Brotli 11 |
|---|---|---|---|
| `release` profile, no wasm-opt | 60.2 MB | 16.6 MB | 9.4 MB |
| `release` profile + `wasm-opt -Oz` (the old `trunk build --release`) | 44.1 MB | 16.2 MB | 9.4 MB |
| `web` profile + one copy of each interface font + `wasm-opt -Oz` (now) | 24.0 MB | 10.0 MB | 6.9 MB |

Fat LTO with one codegen unit makes the final link slower (a clean `web` build of the wasm took
about 3.5 minutes on 8 cores), while `wasm-opt -Oz` gets faster (about 30 s instead of about
3 minutes) because the input is smaller. Trunk runs `wasm-opt` with the wasm features current
Rust enables by default (bulk memory, sign extension and others, `data-wasm-opt-params` in
`index.html`); without them `wasm-opt` rejects the module. Serve the file compressed (see above):
Brotli makes the download about a third of the raw size.

## Embedding in a page (iframe)

```html
<iframe
  src="https://example.com/wordcraft/"
  title="WordCraft image editor"
  style="width: 100%; height: 720px; border: 0;"
  allow="fullscreen; clipboard-read; clipboard-write"
  allowfullscreen>
</iframe>
```

- The app fills the iframe and follows its size, so size the iframe and not the app.
- Keyboard shortcuts go to the iframe after the user clicks into it, as with any embedded app.
- **Cross-origin embeds** work. Preferences are kept in the iframe's `localStorage`. Browsers
  that partition or block third-party storage may forget them between visits, and the app
  then starts with defaults.
- **Sandboxed iframes** need at least
  `sandbox="allow-scripts allow-same-origin allow-downloads allow-popups"`. Without
  `allow-same-origin` there's no storage. Without `allow-downloads`, Save and Export (browser
  downloads) are blocked.
- Don't send `X-Frame-Options: DENY` or a `frame-ancestors` CSP that excludes the embedding page.

## Renderer selection and fallback flags

WordCraft renders with wgpu. It uses **WebGPU** when the browser has it and falls back to
**WebGL2** on its own. URL query flags override this, and they work on the iframe `src` too:

| Flag | Effect |
|---|---|
| *(none)* | WebGPU if available, otherwise WebGL2 |
| `?webgl` | Force the WebGL2 backend (useful when a WebGPU driver misbehaves) |

For example: `<iframe src="https://example.com/wordcraft/?webgl" ...>`.

A browser with neither WebGPU nor WebGL2 gets a message in place of the app.
