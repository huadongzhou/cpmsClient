import { defineConfig } from "vite-plus";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import VueSetupExtend from "unplugin-vue-setup-extend-plus/vite";
import { ElementPlusResolver } from "unplugin-vue-components/resolvers";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => {
  return {
    fmt: {
      ignorePatterns: ["**/*.md", "src/types/auto-imports.d.ts", "src/types/components.d.ts"],
    },
    lint: {
      ignorePatterns: ["src/types/auto-imports.d.ts", "src/types/components.d.ts"],
    },
    staged: {
      "*": "vp check --fix",
    },
    plugins: [
      VueSetupExtend({}),
      vue(),
      UnoCSS(),
      AutoImport({
        imports: ["vue", "pinia", "@vueuse/core"],
        resolvers: [ElementPlusResolver()],
        dts: "src/types/auto-imports.d.ts",
      }),
      Components({
        resolvers: [ElementPlusResolver({ importStyle: "css" })],
        dts: "src/types/components.d.ts",
      }),
    ],
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
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
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
