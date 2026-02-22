import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
    plugins: [sveltekit(), wasm()],
    server: {
        proxy: import.meta.env.PROD ? undefined : {
            "/ffaa.wasm": {
                // Depends on web server running in other dir for now..
                // I'll make some kind of nice build command later..
                target: "http://localhost:3580/target/wasm32-unknown-unknown/release",
            },
        },
    },
});
