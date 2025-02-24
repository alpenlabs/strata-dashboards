# Use Node.js for development
FROM node:20

WORKDIR /app/frontend

# Copy package.json and lock file first to optimize caching
COPY frontend/package.json frontend/package-lock.json ./

# Install dependencies
RUN npm install

# Copy the rest of the project files
COPY frontend .

COPY backend/usage_keys.json public/usage_keys.json

# Expose the Vite development server port (default is 5173)
EXPOSE 5173

# Start the Vite development server
CMD ["npm", "run", "dev", "--", "--host"]
