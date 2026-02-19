#!/usr/bin/env pwsh
Set-Location cc-bot

# .envファイルから環境変数を読み込み
Get-Content ../.env | ForEach-Object {
    if ($_ -match "^([^#][^=]+)=(.*)$") {
        $name = $matches[1].Trim()
        $value = $matches[2].Trim()
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
}

Write-Host "Starting bot..."
Write-Host "GLM API Key: $($env:GLM_API_KEY.Substring(0,10))..."

cargo run
