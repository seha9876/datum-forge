import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import eslintConfigPrettier from "@vue/eslint-config-prettier";
import importPlugin from "eslint-plugin-import";
import vue from "eslint-plugin-vue";
import vueParser from "vue-eslint-parser";

export default [
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "src-tauri/target/**",
      "src-tauri/gen/**"
    ]
  },
  js.configs.recommended,
  ...vue.configs["flat/recommended"],
  {
    files: ["**/*.{ts,vue}"],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        ecmaVersion: "latest",
        parser: tsParser,
        sourceType: "module"
      },
      globals: {
        DragEvent: "readonly",
        window: "readonly",
        document: "readonly",
        console: "readonly",
        Element: "readonly",
        Event: "readonly",
        EventTarget: "readonly",
        HTMLElement: "readonly",
        HTMLSelectElement: "readonly",
        MouseEvent: "readonly",
        PointerEvent: "readonly",
        WheelEvent: "readonly"
      }
    },
    plugins: {
      "@typescript-eslint": tseslint,
      import: importPlugin
    },
    rules: {
      "no-unused-vars": "off",
      "import/order": [
        "error",
        {
          alphabetize: { order: "asc", caseInsensitive: true },
          groups: [
            "builtin",
            "external",
            "internal",
            "parent",
            "sibling",
            "index",
            "type"
          ],
          "newlines-between": "always"
        }
      ],
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_"
        }
      ],
      "vue/multi-word-component-names": "off",
      "vue/no-mutating-props": "off"
    }
  },
  eslintConfigPrettier
];
