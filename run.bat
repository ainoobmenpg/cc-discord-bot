@echo off
REM cc-discord-bot Windows batch script
REM Loads environment variables from .env and runs the bot

REM Check if .env exists in parent directory
if not exist .env (
    echo Error: .env file not found
    echo Please copy .env.example to .env and fill in your credentials
    exit /b 1
)

REM Load environment variables from .env
REM Skip empty lines and comments (lines starting with #)
for /f "usebackq eol=# tokens=1,* delims==" %%a in (".env") do (
    if not "%%a"=="" if not "%%b"=="" (
        set "%%a=%%b"
    )
)

echo Starting bot...

REM Display partial API keys for verification (first 10 characters)
setlocal enabledelayedexpansion
if defined GLM_API_KEY (
    echo GLM API Key: !GLM_API_KEY:~0,10!...
) else (
    echo Warning: GLM_API_KEY is not set
)

if defined DISCORD_BOT_TOKEN (
    echo Discord Token: !DISCORD_BOT_TOKEN:~0,10!...
) else (
    echo Warning: DISCORD_BOT_TOKEN is not set
)
endlocal

REM Change to cc-bot directory and run cargo
cd cc-bot
cargo run
