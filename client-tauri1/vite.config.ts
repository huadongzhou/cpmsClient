import { defineConfig } from "vite-plus";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import VueSetupExtend from "unplugin-vue-setup-extend-plus/vite";
import { ElementPlusResolver } from "unplugin-vue-components/resolvers";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;
const uiRoot = fileURLToPath(new URL("../client-ui", import.meta.url));
const unoConfig = fileURLToPath(new URL("../client-ui/uno.config.ts", import.meta.url));
const shimRoot = fileURLToPath(new URL("./src-shims/tauri", import.meta.url));

export default defineConfig({
  root: uiRoot,
  plugins: [
    VueSetupExtend({}),
    vue(),
    UnoCSS({ configFile: unoConfig }),
    AutoImport({
      imports: ["vue", "pinia", "@vueuse/core"],
      resolvers: [ElementPlusResolver()],
      dts: false,
    }),
    Components({
      resolvers: [ElementPlusResolver({ importStyle: "css" })],
      dts: false,
    }),
  ],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("../client-ui/src", import.meta.url)),
      "@tauri-apps/api/core": `${shimRoot}/core.ts`,
      "@tauri-apps/api/dpi": `${shimRoot}/dpi.ts`,
      "@tauri-apps/api/event": `${shimRoot}/event.ts`,
      "@tauri-apps/api/window": `${shimRoot}/window.ts`,
      "@tauri-apps/api/webviewWindow": `${shimRoot}/webviewWindow.ts`,
    },
  },
  build: {
    outDir: fileURLToPath(new URL("./dist", import.meta.url)),
    emptyOutDir: true,
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["../client-tauri1/src-tauri/**", "../client-tauri2/src-tauri/**"],
    },
  },
});
