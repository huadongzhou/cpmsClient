const path = require("path");
// 不同 unplugin 包的 webpack 入口导出形态不一（有的模块本身即函数，有的在 .default）。
const interop = (m) => (m && m.default) || m;
const AutoImport = interop(require("unplugin-auto-import/webpack"));
const Components = interop(require("unplugin-vue-components/webpack"));
const VueSetupExtend = interop(require("unplugin-vue-setup-extend-plus/webpack"));

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
  // Vue 2.7 + auto-import 下关闭 fork-ts-checker 类型门禁：babel 仅转译，
  // ref/computed 等由 unplugin-auto-import 在打包期注入，运行时正常。
  chainWebpack: (config) => {
    config.plugins.delete("fork-ts-checker");
  },
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
