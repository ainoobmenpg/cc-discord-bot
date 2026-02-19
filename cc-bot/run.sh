#!/bin/bash

export GLM_API_KEY="d881e75411354e9597e0bf22ecde50c8.FxAhdYteOrvBjRG9"
export DISCORD_BOT_TOKEN="MTQ3MzIyMjExMDcyNDc1NTUzOA.Gf-gYm.vR1i4c4txZbWz-7A525wWr6N6UHq4eYtAbC5EQ"

echo "Starting bot..."
echo "GLM API Key: ${GLM_API_KEY:0:10}..."
echo "Discord Token: ${DISCORD_BOT_TOKEN:0:10}..."

cargo run
