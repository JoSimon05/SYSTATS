
import { defineConfig } from "vite"
import { resolve } from "path"

export default defineConfig({
    server: {
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
    build: {
        outDir: "dist",
        emptyOutDir: true,
        rollupOptions: {
            input: {
                index: resolve(import.meta.dirname, "src/index.html"),
                main: resolve(import.meta.dirname, "src/main/main.html"),
            }
        }
    }
})
