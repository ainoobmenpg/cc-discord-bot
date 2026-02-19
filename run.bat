@echo off
cd cc-bot

REM .envファイルから環境変数を読み込み
for /f "usebackq tokens=1,* delims==" %%a in ("../.env") do (
    set "%%a=%%b"
)

echo Starting bot...
echo GLM API Key: %GLM_API_KEY:~0,10%...

cargo run
