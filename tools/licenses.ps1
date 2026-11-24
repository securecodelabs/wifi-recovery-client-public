param([switch]$Check)
$ErrorActionPreference = 'Stop'
$metadataText = & cargo +1.93.0 metadata --locked --format-version 1
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$metadata = $metadataText | ConvertFrom-Json
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add('# Dependency notices')
$lines.Add('')
$lines.Add('Generated from Cargo.lock, including build and test dependencies. Run tools/licenses.ps1 after dependency changes. The executable embeds these notices for offline access with --licenses.')
foreach ($package in ($metadata.packages | Where-Object source | Sort-Object name,version)) {
    $lines.Add('')
    $lines.Add("## $($package.name) $($package.version)")
    $lines.Add('')
    $lines.Add("License: $($package.license)")
    $lines.Add("Source: $($package.repository)")
    $directory = Split-Path -Parent $package.manifest_path
    $files = @(Get-ChildItem -LiteralPath $directory -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)([.-]|$)' })
    if ($package.license_file) { $files += Get-Item -LiteralPath (Join-Path $directory $package.license_file) }
    foreach ($file in ($files | Sort-Object Name -Unique)) {
        $lines.Add('')
        $lines.Add("### $($file.Name)")
        $lines.Add('')
        $lines.Add('```text')
        $lines.Add(([System.IO.File]::ReadAllText($file.FullName)).Replace("`r`n", "`n").TrimEnd())
        $lines.Add('```')
    }
}
$text = ($lines -join "`n") + "`n"
$target = Join-Path $PSScriptRoot '../THIRD_PARTY_NOTICES.md'
if ($Check) {
    if (!(Test-Path -LiteralPath $target) -or [System.IO.File]::ReadAllText($target).Replace("`r`n", "`n") -cne $text) { throw 'Dependency notices are stale' }
} else { [System.IO.File]::WriteAllText($target, $text, [System.Text.UTF8Encoding]::new($false)) }
