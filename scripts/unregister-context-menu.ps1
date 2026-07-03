<#
.SYNOPSIS
    register-context-menu.ps1 で登録した Gitwit のコンテキストメニュー項目を削除する。
#>

$ErrorActionPreference = "SilentlyContinue"

$keys = @(
    "HKCU:\Software\Classes\*\shell\Gitwit",
    "HKCU:\Software\Classes\Directory\shell\Gitwit",
    "HKCU:\Software\Classes\Directory\Background\shell\Gitwit"
)

foreach ($key in $keys) {
    if (Test-Path $key) {
        Remove-Item -Path $key -Recurse -Force
        Write-Host "削除しました: $key"
    } else {
        Write-Host "未登録のためスキップ: $key"
    }
}

Write-Host "`nコンテキストメニューの解除が完了しました。"
