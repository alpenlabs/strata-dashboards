# Use Node.js for development
FROM node:20

WORKDIR /app

# Copy package.json and lock file first to optimize caching
COPY alpen-dashboards/package.json package-lock.json ./

# Install dependencies
RUN npm install

# Copy the rest of the project files
COPY . .

# Copy usage_keys.json if available (from build context)
ARG USAGE_KEYS_PATH
COPY usage_keys.json ${USAGE_KEYS_PATH}

# Ensure the file exists (fallback to empty JSON)
RUN test -f ${USAGE_KEYS_PATH} || echo '{}' > ${USAGE_KEYS_PATH}

# Expose the Vite development server port (default is 5173)
EXPOSE 5173

# Start the Vite development server
CMD ["npm", "run", "start"]