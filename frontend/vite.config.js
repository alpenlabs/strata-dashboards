import { defineConfig } from "vite";

export default defineConfig({
  server: {
    allowedHosts: ["dashboard.testnet.alpenlabs.io"],
    allowedHosts: ["dashboard.testnet-staging.stratabtc.org"],
  },
});

