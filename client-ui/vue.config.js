const path = require("path");
const AutoImport = require("unplugin-auto-import/webpack").default;
const Components = require("unplugin-vue-components/webpack").default;
const VueSetupExtend = require("unplugin-vue-setup-extend-plus/webpack").default;

// LEGACY=1 时按 tauri1（webkit2gtk 2.20 老内核）构建：替换 Tauri v2 API 为 v1 蹼片，
// 产物输出到 client-tauri1/dist；否则常规构建（tauri2 / 独立预览），输出到本包 dist。
const isLegacy = Boolean(process.env.LEGACY);
const shimRoot = path.resolve(__dirname, "../client-tauri1/src-shims/tauri");

const legacyAlias = {
  "@tauri-apps/api/core": path.join(shimRoot, "core.ts"),
  "@tauri-apps/api/dpi": path.join(shimRoot, "dpi.ts"),
  "@tauri-apps/api/event": path.join(shimRoot, "event.ts"),
  "@tauri-apps/api/window": path.join(shimRoot, "window.ts"),
  "@tauri-apps/api/webviewWindow": path.join(shimRoot, "webviewWindow.ts"),
};

module.exports = {
  transpileDependencies: true,
  productionSourceMap: false,
  outputDir: isLegacy ? path.resolve(__dirname, "../client-tauri1/dist") : "dist",
  configureWebpack: {
    plugins: [
      VueSetupExtend(),
      AutoImport({ imports: ["vue", "pinia", "@vueuse/core"], dts: false }),
      Components({ dts: false }),
    ],
    resolve: {
      alias: isLegacy ? legacyAlias : {},
    },
  },
  devServer: {
    port: 1420,
    allowedHosts: "all",
  },
};
