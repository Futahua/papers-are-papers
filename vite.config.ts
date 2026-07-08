import { createHash } from "node:crypto";
import path from "node:path";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

function papersInspectManifest(): Plugin {
  const manifest: Record<string, { file: string; line: number; element: string }> = {};

  return {
    name: "papers-inspect-manifest",
    enforce: "pre",
    transform(source, id) {
      if (!id.endsWith(".tsx") || id.includes("node_modules")) {
        return null;
      }

      const relative = path.relative(process.cwd(), id).replaceAll("\\", "/");
      const transformed = source.replace(
        /<([a-z][a-z0-9-]*)(?=[\s>])/g,
        (match, element: string, offset: number) => {
          const closing = source.indexOf(">", offset);
          const openingTag = source.slice(offset, closing);
          if (openingTag.includes("data-papers-node")) {
            return match;
          }

          const line = source.slice(0, offset).split("\n").length;
          const nodeId = createHash("sha1")
            .update(`${relative}:${line}:${element}`)
            .digest("hex")
            .slice(0, 12);
          manifest[nodeId] = { file: relative, line, element };

          return `<${element} data-papers-node="${nodeId}" data-papers-source="${relative}:${line}"`;
        },
      );

      return { code: transformed, map: null };
    },
    generateBundle() {
      this.emitFile({
        type: "asset",
        fileName: "inspect-manifest.json",
        source: JSON.stringify(manifest, null, 2),
      });
    },
  };
}

export default defineConfig({
  plugins: [papersInspectManifest(), react()],
  clearScreen: false,
  // Multi-page: main app + the isolated provider key-entry window.
  // The key-entry window ships its own minimal bundle so the main app's
  // React state never holds an API key.
  build: {
    rollupOptions: {
      input: {
        main: path.resolve(process.cwd(), "index.html"),
        keyEntry: path.resolve(process.cwd(), "index-key-entry.html"),
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/launcher/**"],
    },
  },
});
