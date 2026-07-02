<#
.SYNOPSIS
    Windows Explorer の右クリックメニューに Gitwit の「履歴を表示」項目を登録する。

.DESCRIPTION
    HKEY_CURRENT_USER (現在のユーザー) 配下のみを操作するため、管理者権限は不要。
    - ファイル右クリック時   : *\shell\Gitwit
    - フォルダ右クリック時   : Directory\shell\Gitwit
    - フォルダ背景の右クリック時 : Directory\Background\shell\Gitwit

    登録後、対象を右クリックすると「Gitwitで履歴を表示」というメニューが表示され、
    クリックすると対象ファイル/フォルダを引数に Gitwit が起動する。
#>

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exePath = Join-Path $scriptDir "..\target\release\gitwit.exe" | Resolve-Path -ErrorAction SilentlyContinue

if (-not $exePath) {
    Write-Error "gitwit.exe が見つかりません。先に 'cargo build --release' を実行してください。"
    exit 1
}

$exePath = $exePath.Path
$menuLabel = "Gitwitで履歴を表示"

function Register-ContextMenuEntry {
    param(
        [string]$KeyPath,
        [string]$CommandArg
    )

    $shellKey = Join-Path $KeyPath "Gitwit"
    $commandKey = Join-Path $shellKey "command"

    New-Item -Path $shellKey -Force | Out-Null
    Set-ItemProperty -Path $shellKey -Name "MUIVerb" -Value $menuLabel
    Set-ItemProperty -Path $shellKey -Name "Icon" -Value "`"$exePath`""

    New-Item -Path $commandKey -Force | Out-Null
    Set-ItemProperty -Path $commandKey -Name "(Default)" -Value "`"$exePath`" `"$CommandArg`""

    Write-Host "登録しました: $shellKey"
}

# ファイルを右クリックしたとき
Register-ContextMenuEntry -KeyPath "HKCU:\Software\Classes\*\shell" -CommandArg "%1"

# フォルダ自体を右クリックしたとき
Register-ContextMenuEntry -KeyPath "HKCU:\Software\Classes\Directory\shell" -CommandArg "%1"

# フォルダの背景(空白部分)を右クリックしたとき
Register-ContextMenuEntry -KeyPath "HKCU:\Software\Classes\Directory\Background\shell" -CommandArg "%V"

Write-Host "`nコンテキストメニューの登録が完了しました。Explorerで右クリックして確認してください。"
Write-Host "解除する場合は unregister-context-menu.ps1 を実行してください。"
