import { defineConfig } from "vite";

export default defineConfig({
  server: {
    allowedHosts: [
      "dashboard.testnet.alpenlabs.io",
      "dashboard.testnet-staging.stratabtc.org",
      "dashboard.development.stratabtc.org",
    ]
  },
});

