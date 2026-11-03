#!/usr/bin/env bash
# Build the browser version and zip it:  $DIST/wordcraft-web-<version>.zip
#
# Usage: packaging/web/package.sh [--skip-build]
#
# Needs: trunk (brew install trunk / cargo install trunk --locked) and the wasm32-unknown-unknown
# target. The zip holds a self-contained static site in wordcraft-web-<version>/ that works
# from any URL path and inside an <iframe>. Hosting notes: packaging/web/README.md.
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/web"

if [[ "${1:-}" != "--skip-build" ]]; then
  command -v trunk >/dev/null || { echo "error: trunk not found (cargo install trunk --locked)" >&2; exit 1; }
  # The `web` profile (Cargo.toml) builds for size: about 20 MB of wasm instead of about 44 MB.
  (cd "$ROOT/apps/wordcraft-web" && trunk build --release --cargo-profile web)
fi

SITE="$ROOT/dist/web"
[[ -f "$SITE/index.html" ]] || { echo "error: $SITE/index.html missing; run without --skip-build" >&2; exit 1; }
# Paths must be relative so the site works under any prefix (public_url = "./" in Trunk.toml).
if grep -Eq '(src|href)="/[^/]' "$SITE/index.html"; then
  echo "error: $SITE/index.html has root-absolute URLs; it would break when served from a sub-path" >&2
  exit 1
fi

NAME="wordcraft-web-$VERSION"
WORK="$CARGO_TARGET_DIR/web-package"
rm -rf "$WORK"
mkdir -p "$WORK/$NAME"
cp -R "$SITE/." "$WORK/$NAME/"
# fonts/ holds every bundled font (Trunk copies the folder), but the wasm only fetches the ones
# index.html preloads; the others are compiled in. Keep those and the licences.
for f in "$WORK/$NAME"/fonts/*.ttf; do
  grep -q "href=\"fonts/$(basename "$f")\"" "$SITE/index.html" || rm "$f"
done
# Precompressed copies for servers that send them as they are (nginx gzip_static/brotli_static,
# Caddy precompressed); the others ignore them. The originals stay for clients without either.
for f in "$WORK/$NAME"/*.wasm "$WORK/$NAME"/*.js "$WORK/$NAME"/index.html "$WORK/$NAME"/fonts/*.ttf "$WORK/$NAME"/egui-fonts/*.ttf; do
  gzip -k9n "$f"
  if command -v brotli >/dev/null; then brotli -kq 11 "$f"; fi
done
# Sample server configs (MIME type, caching, compression); harmless where unused.
cp "$HERE/_headers" "$HERE/.htaccess" "$WORK/$NAME/"
cp "$HERE/README.md" "$WORK/$NAME/HOSTING.md"
copy_docs "$WORK/$NAME"
rm -f "$DIST/$NAME.zip"
(cd "$WORK" && zip -qr9 "$DIST/$NAME.zip" "$NAME")
echo "wrote $DIST/$NAME.zip"
ls -lh "$DIST/$NAME.zip"
