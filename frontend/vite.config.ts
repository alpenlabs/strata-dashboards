import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [react()],
    optimizeDeps: {
        include: ["@tanstack/react-query"],
    },
    server: {
        allowedHosts: [
            "dashboard.testnet.alpenlabs.io",
            "dashboard.testnet-staging.stratabtc.org",
            "dashboard.development.stratabtc.org",
        ],
    },
});
