import {
  defineConfig,
  presetAttributify,
  presetIcons,
  presetTypography,
  presetUno,
  presetWebFonts,
  transformerDirectives,
  transformerVariantGroup,
} from "unocss";

export default defineConfig({
  content: {
    pipeline: {
      exclude: ["node_modules", ".git", "dist", "src/libs/**", "**/*.bundle.js", "**/*.umd.js"],
    },
  },
  theme: {
    colors: {
      primary: "var(--el-color-primary)",
      primary_dark: "var(--el-color-primary-light-5)",
    },
  },
  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      scale: 1.0,
      warn: false,
    }),
    presetTypography(),
    presetWebFonts({
      fonts: {},
    }),
  ],
  transformers: [transformerDirectives(), transformerVariantGroup()],
});
