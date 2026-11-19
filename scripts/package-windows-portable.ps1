[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $BinaryPath,

    [Parameter(Mandatory = $true)]
    [string] $LicensePath,

    [Parameter(Mandatory = $true)]
    [string] $OutputDirectory,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$')]
    [string] $Version
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$binary = (Resolve-Path (Join-Path $repoRoot $BinaryPath)).Path
$license = (Resolve-Path (Join-Path $repoRoot $LicensePath)).Path
$configPath = Join-Path $repoRoot 'src-tauri/tauri.conf.json'
$config = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json

if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "Portable executable not found: $binary"
}
if (-not (Test-Path -LiteralPath $license -PathType Leaf)) {
    throw "License file not found: $license"
}

# Validate the PE machine field instead of trusting the executable filename.
$peStream = [System.IO.File]::OpenRead($binary)
$peReader = [System.IO.BinaryReader]::new($peStream)
try {
    if ($peReader.ReadUInt16() -ne 0x5A4D) {
        throw 'Portable executable has no DOS/PE header.'
    }
    $peStream.Position = 0x3c
    $peOffset = $peReader.ReadInt32()
    if ($peOffset -lt 0 -or $peOffset -gt ($peStream.Length - 6)) {
        throw 'Portable executable has an invalid PE header offset.'
    }
    $peStream.Position = $peOffset
    if ($peReader.ReadUInt32() -ne 0x00004550 -or $peReader.ReadUInt16() -ne 0x8664) {
        throw 'Portable executable is not Windows x64 (AMD64).'
    }
}
finally {
    $peReader.Dispose()
}

# A portable executable can only omit extra payload if Tauri has no sidecars or
# runtime resources besides the license that this script copies beside it.
$bundle = $config.bundle
if ($bundle.externalBin -and @($bundle.externalBin).Count -gt 0) {
    throw 'Portable package must explicitly include configured Tauri externalBin sidecars.'
}
$resourceEntries = @($bundle.resources.PSObject.Properties)
if ($resourceEntries.Count -gt 1 -or ($resourceEntries.Count -eq 1 -and
        ($resourceEntries[0].Name -ne '../LICENSE' -or $resourceEntries[0].Value -ne 'LICENSE'))) {
    throw 'Portable package must explicitly include any Tauri resources beyond ../LICENSE.'
}

$outputDirectoryPath = Join-Path $repoRoot $OutputDirectory
New-Item -ItemType Directory -Path $outputDirectoryPath -Force | Out-Null
$archive = Join-Path $outputDirectoryPath "EasyMD_${Version}_x64-portable.zip"
if (Test-Path -LiteralPath $archive) {
    throw "Refusing to overwrite existing portable archive: $archive"
}

$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("EasyMD-portable-" + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'EasyMD.exe')
    Copy-Item -LiteralPath $license -Destination (Join-Path $stage 'LICENSE')
    @'
EasyMD Windows x64 Portable / Windows x64 便携版

1. Extract this ZIP to a folder you can write to. / 解压到有写入权限的文件夹。
2. Run EasyMD.exe. No installer is needed. / 运行 EasyMD.exe，无需安装。

Requirements: Windows x64 and the Microsoft Edge WebView2 Runtime. Windows 11
includes WebView2; on Windows 10, install the Evergreen WebView2 Runtime if it
is not already present.
运行要求：Windows x64 和 Microsoft Edge WebView2 Runtime。Windows 11 已包含
WebView2；Windows 10 若尚未安装，请安装 Evergreen WebView2 Runtime。

This ZIP is not a data-portable mode. EasyMD settings, indexes, and other
application configuration remain in the current user's AppData directory;
they do not move with this folder. Your Markdown vault stays wherever you
choose to keep it.
此 ZIP 不表示数据会随程序移动。EasyMD 设置、索引和其他应用配置仍保存在当前
用户的 AppData 目录，不会随本文件夹搬移。Markdown 知识库仍保存在你选择的位置。

See LICENSE for the MIT license. / MIT 许可证全文见 LICENSE。
'@ | Set-Content -LiteralPath (Join-Path $stage 'README-portable.txt') -Encoding utf8
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        $stage,
        $archive,
        [System.IO.Compression.CompressionLevel]::Optimal,
        $false
    )

    $zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
    try {
        $entryNames = @($zip.Entries | ForEach-Object { $_.FullName })
        foreach ($requiredEntry in @('EasyMD.exe', 'LICENSE', 'README-portable.txt')) {
            if ($entryNames -notcontains $requiredEntry) {
                throw "Portable archive is missing required entry: $requiredEntry"
            }
        }
        if ($entryNames.Count -ne 3) {
            throw "Portable archive contains unexpected entries: $($entryNames -join ', ')"
        }
        $archiveLicense = $zip.GetEntry('LICENSE')
        $archiveLicenseStream = $archiveLicense.Open()
        $sourceLicenseBytes = [System.IO.File]::ReadAllBytes($license)
        $archiveLicenseBytes = [byte[]]::new([int]$archiveLicense.Length)
        try {
            $read = 0
            while ($read -lt $archiveLicenseBytes.Length) {
                $count = $archiveLicenseStream.Read($archiveLicenseBytes, $read, $archiveLicenseBytes.Length - $read)
                if ($count -eq 0) { break }
                $read += $count
            }
        }
        finally {
            $archiveLicenseStream.Dispose()
        }
        if ($read -ne $archiveLicenseBytes.Length -or
            [Convert]::ToBase64String($sourceLicenseBytes) -ne [Convert]::ToBase64String($archiveLicenseBytes)) {
            throw 'Portable archive LICENSE does not match the repository LICENSE.'
        }
    }
    finally {
        $zip.Dispose()
    }
    Write-Output "Created $archive"
}
finally {
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
}
