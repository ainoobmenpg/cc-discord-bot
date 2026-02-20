#!/usr/bin/env pwsh
# cc-discord-bot Windows PowerShell script
# Loads environment variables from .env and runs the bot

# Check if .env exists
if (-not (Test-Path .env)) {
    Write-Host "Error: .env file not found" -ForegroundColor Red
    Write-Host "Please copy .env.example to .env and fill in your credentials" -ForegroundColor Yellow
    exit 1
}

# Load environment variables from .env
# Skip empty lines and comments (lines starting with #)
Get-Content .env | ForEach-Object {
    $line = $_.Trim()
    if ($line -and -not $line.StartsWith("#")) {
        if ($line -match "^([^=]+)=(.*)$") {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            # Remove surrounding quotes if present
            if ($value.StartsWith('"') -and $value.EndsWith('"')) {
                $value = $value.Substring(1, $value.Length - 2)
            }
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
}

Write-Host "Starting bot..." -ForegroundColor Green

# Display partial API keys for verification (first 10 characters)
if ($env:GLM_API_KEY) {
    Write-Host "GLM API Key: $($env:GLM_API_KEY.Substring(0, [Math]::Min(10, $env:GLM_API_KEY.Length)))..."
} else {
    Write-Host "Warning: GLM_API_KEY is not set" -ForegroundColor Yellow
}

if ($env:DISCORD_BOT_TOKEN) {
    Write-Host "Discord Token: $($env:DISCORD_BOT_TOKEN.Substring(0, [Math]::Min(10, $env:DISCORD_BOT_TOKEN.Length)))..."
} else {
    Write-Host "Warning: DISCORD_BOT_TOKEN is not set" -ForegroundColor Yellow
}

# Change to cc-bot directory and run cargo
Set-Location cc-bot
cargo run
