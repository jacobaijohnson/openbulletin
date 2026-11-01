# End-to-end music check against a staged Windows bundle folder.
#
# Runs the staged ob-verovio-helper.exe (which sits beside the
# staged verovio-python/ folder, exactly as in the shipped bundle)
# on a small MusicXML sample and asserts the engraved answer:
# ok, a Verovio backend, at least one system, every system's
# measured staff at or above the 5.0 mm legibility floor
# (crates/ob-music/src/render.rs, FLOOR_STAFF_MM), at the 6.5 mm
# target. This is the stock-Windows proof: PATH is scrubbed to
# the Windows system folders and the OB_VEROVIO_* overrides are
# removed first, so no developer Python, no PATH wheel, and no
# env-var rescue can stand in for a broken staged layout. On a
# stock runner the only Python that can answer is the staged one
# the helper probes beside its own exe.
param(
    [Parameter(Mandatory = $true)][string]$BundleDir,
    [Parameter(Mandatory = $true)][string]$HelperExe,
    [Parameter(Mandatory = $true)][string]$MusicXml
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

if (-not (Test-Path $HelperExe)) {
    throw "The helper to check is missing: $HelperExe"
}
if (-not (Test-Path (Join-Path $BundleDir 'verovio-python/python.exe'))) {
    throw 'The staged verovio-python/python.exe is missing; the bundle must not ship like this.'
}

# Stock-machine simulation: no overrides, no developer Python.
Remove-Item Env:OB_VEROVIO_BIN -ErrorAction SilentlyContinue
Remove-Item Env:OB_VEROVIO_PYTHON -ErrorAction SilentlyContinue
$env:PATH = 'C:\Windows\System32;C:\Windows'

$xml = Get-Content $MusicXml -Raw
$request = @{
    protocol  = 1
    command   = 'render'
    musicxml  = $xml
    width_mm  = 170.0
    staff_mm  = 6.5
} | ConvertTo-Json -Compress

# Pipe bytes, not PowerShell strings, into the helper: a string
# pipe would re-encode the request behind our back.
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $HelperExe
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$proc = [System.Diagnostics.Process]::Start($psi)
$bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
$proc.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
$proc.StandardInput.Close()
$out = $proc.StandardOutput.ReadToEnd()
$proc.WaitForExit()
$err = $proc.StandardError.ReadToEnd()
if ($proc.ExitCode -ne 0) {
    throw "The staged helper exited $($proc.ExitCode): $err"
}
$resp = $out | ConvertFrom-Json
if (-not $resp.ok) {
    throw "The staged engraver refused the sample: $($resp.kind) $($resp.error)"
}
if ($resp.backend -notmatch 'Verovio') {
    throw "The staged engraver answered without a Verovio backend: $($resp.backend)"
}
if ($resp.systems.Count -lt 1) {
    throw 'The staged engraver drew no systems from the sample.'
}
foreach ($system in $resp.systems) {
    if ($system.staff_mm -lt 5.0) {
        throw "The staged engraver drew a staff below the 5.0 mm floor: $($system.staff_mm) mm."
    }
}
Write-Host "Staged bundle music check: backend $($resp.backend), $($resp.systems.Count) system(s), staff $($resp.systems[0].staff_mm) mm (floor 5.0 mm), noteheads $($resp.systems[0].noteheads)."
