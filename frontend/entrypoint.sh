#!/bin/sh

# Copy usage_keys.json if it exists at runtime
if [ -f "/mnt/usage_keys.json" ]; then
  cp /mnt/usage_keys.json /app/public/usage_keys.json
else
  echo '{}' > /app/public/usage_keys.json  # Ensure it exists with default data
fi

# Start the frontend
exec npm run start
