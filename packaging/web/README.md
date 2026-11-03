# Hosting WordCraft for the web

`wordcraft-web-<version>.zip` (from the GitHub release, or `packaging/web/package.sh`) holds a
static site in `wordcraft-web-<version>/`:

| File | What it is |
|---|---|
| `index.html` | The page. It loads everything through relative URLs. |
| `wordcraft-web-<hash>.js` | wasm-bindgen glue (generated, ES module) |
| `wordcraft-web-<hash>_bg.wasm` | The app, about 15 MB, or 6.2 MB with gzip and 4.6 MB with Brotli |
| `fonts/` | Document fonts the app fetches after it starts (Source Sans 3 bold/italic, Source Serif 4, Inter Medium/SemiBold, JetBrains Mono) and their licences |
| `egui-fonts/` | egui's default interface faces (Hack, Ubuntu Light, emoji, icons), fetched after start-up |
| `*.br`, `*.gz` | Precompressed copies of the files above, for servers that can send them (optional) |
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
- **Compression:** turn on gzip or Brotli for `.wasm`, `.js`, `.html` and `.ttf`. That takes the
  download from about 20 MB to about 8.5 MB (gzip) or 6.5 MB (Brotli). The zip already holds
  `.br` and `.gz` copies made at the highest levels, so servers that send precompressed files
  (nginx `gzip_static on;` / `brotli_static on;`, Caddy `file_server { precompressed br gzip }`)
  don't have to compress anything. Delete them if your host compresses on its own.
- **Fonts:** keep `fonts/` and `egui-fonts/` next to `index.html`. The app starts without them,
  with only Inter Regular and Source Sans 3 Regular compiled in, then fetches the other faces in
  the background and lays the document out again when they arrive. If they can't be fetched,
  bold, italic and serif text falls back to the compiled-in faces, medium/semibold interface
  text to Inter Regular, and symbols and emoji only egui's faces have draw as boxes.
- **Caching:** `Cache-Control: public, max-age=31536000, immutable` on the hashed `.wasm` and
  `.js` files, a shorter lifetime on `fonts/` and `egui-fonts/` (the names carry no hash), and
  `no-cache` on `index.html`.
- **HTTPS:** WebGPU (and the clipboard) only work in a secure context, which means `https://`
  or `http://localhost`. Over plain HTTP elsewhere, the app falls back to WebGL2.
- **No special isolation headers:** WordCraft doesn't use `SharedArrayBuffer`, so it doesn't
  need `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy`. If your site already sends
  COEP `require-corp`, also send `Cross-Origin-Resource-Policy: same-origin` (or `cross-origin`
  when the files live on a CDN) on the app's files.

### nginx

[`nginx/wordcraft-viewer.conf`](nginx/wordcraft-viewer.conf) (`nginx-example.conf` in the web zip)
is a complete, commented server block for this folder: point `root` at it, set `server_name` and
the certificate, and drop it into `/etc/nginx/conf.d/`. It

- adds `application/wasm` and `font/ttf` to `mime.types`;
- sends the `.br`/`.gz` copies with `gzip_static on;` and `brotli_static on;`. Brotli needs the
  ngx_brotli static module (Debian/Ubuntu `libnginx-mod-http-brotli-static`); without it, delete
  the `brotli_static` line and gzip is used;
- caches the hashed `wordcraft-web-<hash>.js`, `wordcraft-web-<hash>_bg.wasm` and icon for a year
  (`immutable`), `fonts/` and `egui-fonts/` for a week, and revalidates `index.html` (`no-cache`);
- redirects plain HTTP to HTTPS (WebGPU, the clipboard and the save picker need a secure
  context);
- shows, commented out, the same rules under a sub-path (`location /wordcraft/` with `alias`):
  every URL in `index.html` is relative (Trunk `public_url` is `./`), so nothing else changes.

[`nginx/documents-cors.conf`](nginx/documents-cors.conf) (`nginx-documents-cors.conf` in the zip)
is for a *different* server that hosts documents opened with `?url=`/`?src=` from another origin:
it sends `Access-Control-Allow-Origin` for the viewer's origin, allows `GET` and `OPTIONS`
(answering `OPTIONS` with 204), and gives `.docx` its MIME type
`application/vnd.openxmlformats-officedocument.wordprocessingml.document` (and `.odt`, `.rtf`,
`.doc`…). Both files pass `nginx -t` with nginx 1.26 (Debian 13), with and without the brotli
module.

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
| `web` profile + one copy of each interface font + `wasm-opt -Oz` | 24.0 MB | 10.0 MB | 6.9 MB |
| + viewer only (no `editor` feature), document fonts fetched | 19.9 MB | 8.3 MB | 6.0 MB |
| + viewer-only command handlers, egui's default faces fetched | 17.5 MB | 7.0 MB | 5.0 MB |
| + SIMD128 | 16.0 MB | 6.7 MB | 4.9 MB |
| + Inter Medium/SemiBold and JetBrains Mono fetched (now) | 15.0 MB | 6.2 MB | 4.6 MB |

The web build is the WordCraft Viewer: it depends on `wordcraft-ui-egui` without its default
`editor` feature, so the editing dialogs and commands are compiled out (`docs/viewer.md`). Eight
document faces, three interface faces and egui's four default faces (about 4.7 MB, 1.9 MB with
Brotli) are not in the wasm; the app fetches them from `fonts/` and `egui-fonts/` after it starts
(`index.html` preloads the interface ones), so they don't hold up the first frame. With them the
whole download is 19.8 MB raw, 8.5 MB gzip, 6.5 MB Brotli.

Fat LTO with one codegen unit makes the final link slower (a clean `web` build of the wasm took
about 3.5 minutes on 8 cores), while `wasm-opt -Oz` gets faster (about 30 s instead of about
3 minutes) because the input is smaller. Trunk runs `wasm-opt` with the wasm features current
Rust enables by default (bulk memory, sign extension and others, `data-wasm-opt-params` in
`index.html`); without them `wasm-opt` rejects the module. Serve the file compressed (see above):
Brotli makes the download about a third of the raw size.

## Opening a document from an address

`?url=` (or its alias `?src=`; `?url=` wins if both are given) opens a document when the page
starts, as if it had been picked in File › Open:

```
https://example.com/wordcraft/?url=/files/report.docx
https://example.com/wordcraft/?url=https%3A%2F%2Fcdn.example.org%2Fminutes.odt&name=Minutes.odt
<iframe src="https://example.com/wordcraft/?src=../docs/handbook.docx" ...>
```

- The address may be relative (resolved against the viewer page) or absolute. Only `http:`,
  `https:` and `blob:` addresses are fetched; `javascript:`, `data:`, `file:` and others are
  refused. Percent-encode an address that has its own `?` or `&` (`encodeURIComponent`).
- The document is named after `?name=` if given, else the address's last path segment
  (decoded, without the query), else `document.docx`. The name's extension picks the importer;
  without one the format is told from the content.
- Cookies are sent only to the viewer's own origin (`credentials: same-origin`).
- **CORS:** a file on another origin is readable only if its server sends
  `Access-Control-Allow-Origin` naming the viewer's origin (or `*`), for example in nginx
  `add_header Access-Control-Allow-Origin https://example.com;` (a full example:
  [`nginx/documents-cors.conf`](nginx/documents-cors.conf)). Without it the browser blocks the
  download and the viewer says so.
- While it downloads, the viewer shows *Opening report.docx…*. A network or CORS failure, an HTTP
  error (404…) or a file WordCraft can't open is shown on the page and on the status bar and
  logged to the console. Documents larger than 512 MB are refused.
- `?url=` takes precedence over `?sample`, and is ignored with `?host=parent` (the parent page
  sends the document then).

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
| `?sample` | Open the sample document |
| `?url=ADDRESS` / `?src=ADDRESS` (`&name=NAME`) | Open the document at that address (see *Opening a document from an address*) |

For example: `<iframe src="https://example.com/wordcraft/?webgl" ...>`.

A browser with neither WebGPU nor WebGL2 gets a message in place of the app.
