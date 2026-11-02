# Stage the Verovio music engraver into a Windows bundle folder.
#
# The bundle ships the engraver the way the helper already probes for
# it (crates/ob-music/src/engine.rs, find_backend): a `verovio-python/`
# folder beside the exe, holding a Python runtime plus the official
# Verovio wheel. No build from source: both artifacts are official,
# unmodified, and checked against pinned SHA-256 hashes before
# anything is staged, in the same fetch -> hash-check -> stage ->
# named-file-assertion pattern the bundle workflow uses for PDFium.
# If any step fails, the script stops loudly and stages nothing
# half-done: a green run never ships a bundle quietly missing its
# engraver, its license, or its source offer.
#
# Pins (checked 2026-11-01):
# - Python 3.14.10 embeddable amd64 zip, from python.org's FTP
#   layout. Hash confirmed against a local download on 2026-11-01.
# - Verovio 6.4.0 cp310-abi3 win_amd64 wheel, from PyPI. Hash from
#   PyPI's release JSON on 2026-11-01; the manylinux sibling
#   wheel's hash from the same JSON was independently confirmed
#   against a local download the same day. The abi3 wheel runs
#   on any CPython >= 3.10, so it pairs with the 3.14 runtime.
# - The LGPL-3.0 text is NOT downloaded separately: it is copied
#   out of the wheel itself (verovio-6.4.0.dist-info/licenses/
#   COPYING.LESSER), so the staged license is the artifact's own.
#   If that file ever moves inside the wheel, this script fails
#   instead of shipping a bundle without a license.
#
# Used by .github/workflows/windows-bundle.yml (the real bundle)
# and by .github/workflows/ci.yml (the Windows leg's staged-layout
# end-to-end check), so both prove the same staging shape.
param(
    [Parameter(Mandatory = $true)][string]$BundleDir
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$PythonVersion = '3.14.10'
$PythonUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
$PythonSha256 = '90905e64a840557d8c1bc4f5b74db3d7ee85e3ffb217611cac1c7813acea4159'
$VerovioVersion = '6.4.0'
$WheelFile = "verovio-$VerovioVersion-cp310-abi3-win_amd64.whl"
$WheelUrl = "https://files.pythonhosted.org/packages/e9/a3/fe81ca815c9d84834d155450dc4d70618c7ea78a04bb9d28ecdbbdb5a5f2/$WheelFile"
$WheelSha256 = 'd81bee29b83978a48a065710cb16ebba0914c9c2212305e8c154a6f05d864424'

function Assert-Hash([string]$Path, [string]$Expected, [string]$What) {
    $actual = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $Expected) {
        throw "$What does not match its recorded hash. Got $actual, expected $Expected."
    }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) 'ob-verovio-stage'
if (Test-Path $work) { Remove-Item $work -Recurse -Force }
New-Item -ItemType Directory -Force $work | Out-Null

$pythonDir = Join-Path $BundleDir 'verovio-python'
if (Test-Path $pythonDir) { Remove-Item $pythonDir -Recurse -Force }
New-Item -ItemType Directory -Force $pythonDir | Out-Null

# 1. The Python runtime (embeddable distribution: no install, no
# registry, nothing outside this folder).
$pythonZip = Join-Path $work 'python-embed.zip'
Invoke-WebRequest -Uri $PythonUrl -OutFile $pythonZip
Assert-Hash $pythonZip $PythonSha256 'The Python runtime download'
Expand-Archive -Path $pythonZip -DestinationPath $pythonDir -Force

# 2. The Verovio wheel, unpacked (wheels are zips) into the
# runtime's site-packages. No pip: the embeddable distribution
# ships without it, and unpacking is all the wheel needs.
$sitePackages = Join-Path $pythonDir 'Lib/site-packages'
New-Item -ItemType Directory -Force $sitePackages | Out-Null
$wheel = Join-Path $work $WheelFile
Invoke-WebRequest -Uri $WheelUrl -OutFile $wheel
Assert-Hash $wheel $WheelSha256 'The Verovio wheel download'
Expand-Archive -Path $wheel -DestinationPath $sitePackages -Force

# 3. Teach the embeddable runtime where the wheel lives: its
# `pythonXY._pth` file lists the import path, and site-packages
# is not in it by default.
$pth = Get-Item (Join-Path $pythonDir 'python*._pth') | Select-Object -First 1
$pthLines = Get-Content $pth.FullName
if ($pthLines -notcontains 'Lib/site-packages') {
    $pthLines += 'Lib/site-packages'
    Set-Content -Path $pth.FullName -Value $pthLines
}

# 4. The license text, from the wheel itself, beside the exe.
$licenseInWheel = Join-Path $sitePackages "verovio-${VerovioVersion}.dist-info/licenses/COPYING.LESSER"
if (-not (Test-Path $licenseInWheel)) {
    throw "The Verovio wheel no longer carries dist-info/licenses/COPYING.LESSER; the bundle must not ship without its LGPL-3.0 text."
}
$licenseOut = Join-Path $BundleDir 'Verovio-LGPL-3.0.txt'
Copy-Item $licenseInWheel $licenseOut
if ((Get-Item $licenseOut).Length -eq 0) {
    throw 'Verovio-LGPL-3.0.txt staged empty; the bundle must not ship like this.'
}

# 5. The source offer, beside the exe.
@"
Verovio $VerovioVersion — source offer
======================================

The music engraver in the verovio-python folder is Verovio
$VerovioVersion, licensed under the GNU Lesser General Public
License version 3 (see Verovio-LGPL-3.0.txt beside this note).
It is the unmodified official wheel from PyPI:

  https://pypi.org/project/verovio/$VerovioVersion/

The complete corresponding source is available from Verovio's
own repository, release tag v${VerovioVersion}:

  https://github.com/rism-digital/verovio

Nothing of Verovio is linked into OpenBulletin or modified by
it: it runs out-of-process, behind our own helper binary
(ob-verovio-helper.exe), which speaks to the app over
versioned JSON.
"@ | Out-File -Encoding utf8 (Join-Path $BundleDir 'Verovio-SOURCE.txt')

# 6. Prove the staged layout imports before anyone ships it:
# the staged python, the staged wheel, no PATH python involved.
$pythonExe = Join-Path $pythonDir 'python.exe'
if (-not (Test-Path $pythonExe)) {
    throw 'verovio-python/python.exe is missing after staging.'
}
$probe = & $pythonExe -c "import verovio; print(verovio.toolkit().getVersion())"
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($probe)) {
    throw 'The staged Python cannot import the staged Verovio wheel.'
}
Write-Host "Staged Verovio probe: $probe"
Get-ChildItem $pythonDir | Select-Object Name, Length
