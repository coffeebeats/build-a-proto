# This script installs 'baproto' by downloading prebuilt binaries from the
# project's GitHub releases page. By default the latest version is installed,
# but a different release can be used instead by setting $BAPROTO_VERSION.
#
# The script will set up a 'baproto' cache at '%LOCALAPPDATA%/baproto'. This
# behavior can be customized by setting '$BAPROTO_HOME' prior to running the
# script. Note that this script will overwrite any 'baproto' binary artifacts
# in '$BAPROTO_HOME/bin'.
#
# NOTE: Unlike the 'install.sh' counterpart, this script exclusively installs
# the 'baprotoc' binary for 64-bit Windows. If an alternative 'baproto' binary is
# required, follow the documentation for an alternative means of installation:
# https://github.com/coffeebeats/baproto/blob/v0.2.4/docs/installation.md # x-release-please-version

<#
.SYNOPSIS
  Install 'baproto' for compiling custom binary encodings into language-specific bindings.

.DESCRIPTION
  This script downloads the specified version of 'baproto' from GitHub, extracts
  its artifacts to the 'baproto' store ('$BAPROTO_HOME' or a default path), and then
  updates environment variables as needed.

.PARAMETER NoModifyPath
  Do not modify the $PATH environment variable.

.PARAMETER Version
  Install the specified version of 'baproto'.

.INPUTS
  None

.OUTPUTS
  $env:BAPROTO_HOME\bin\baproto.exe

.NOTES
  Version:        0.2.4 # x-release-please-version
  Author:         https://github.com/coffeebeats

.LINK
  https://github.com/coffeebeats/build-a-proto
#>

# ------------------------------ Define: Params ------------------------------ #

Param (
  # NoModifyPath - if set, the user's $PATH variable won't be updated
  [Switch] $NoModifyPath = $False,

  # Version - override the specific version of 'baproto' to install
  [String] $Version = "v0.2.4" # x-release-please-version
)

# ------------------------- Function: Get-BaProtoHome ------------------------- #

# Returns the current value of the 'BAPROTO_HOME' environment variable or a
# default if unset.
Function Get-BaProtoHome() {
  if ([string]::IsNullOrEmpty($env:BAPROTO_HOME)) {
    return Join-Path -Path $env:LOCALAPPDATA -ChildPath "baproto"
  }

  return $env:BAPROTO_HOME
}

# ------------------------ Function: Get-BaProtoVersion ----------------------- #

Function Get-BaProtoVersion() {
  if ([string]::IsNullOrEmpty($env:BAPROTO_VERSION)) {
    return "v" + $Version.TrimStart("v")
  }

  return $env:BAPROTO_VERSION
}

# --------------------- Function: Create-Temporary-Folder -------------------- #

# Creates a new temporary directory for downloading and extracting 'baproto'. The
# returned directory path will have a randomized suffix.
Function New-TemporaryFolder() {
  # Make a new temporary folder with a randomized suffix.
  return New-Item `
    -ItemType Directory `
    -Name "baproto-$([System.IO.Path]::GetFileNameWithoutExtension([System.IO.Path]::GetRandomFileName()))"`
    -Path $env:temp
}

# ------------------------------- Define: Store ------------------------------ #

$BaProtoHome = Get-BaProtoHome

Write-Host "info: setting 'BAPROTO_HOME' environment variable: ${BaProtoHome}"

[System.Environment]::SetEnvironmentVariable("BAPROTO_HOME", $BaProtoHome, "User")

# ------------------------------ Define: Version ----------------------------- #
  
$BaProtoVersion = Get-BaProtoVersion

$BaProtoArchive = "baproto-${BaProtoVersion}-windows-x86_64.zip"

# ----------------------------- Execute: Install ----------------------------- #
  
$BaProtoRepositoryURL = "https://github.com/coffeebeats/build-a-proto"

# Install downloads 'baproto' and extracts its binaries into the store. It also
# updates environment variables as needed.
Function Install() {
  $BaProtoTempFolder = New-TemporaryFolder

  $BaProtoArchiveURL = "${BaProtoRepositoryURL}/releases/download/${BaProtoVersion}/${BaProtoArchive}"
  $BaProtoDownloadTo = Join-Path -Path $BaProtoTempFolder -ChildPath $BaProtoArchive

  $BaProtoHomeBinPath = Join-Path -Path $BaProtoHome -ChildPath "bin"

  try {
    Write-Host "info: installing version: '${BaProtoVersion}'"

    Invoke-WebRequest -URI $BaProtoArchiveURL -OutFile $BaProtoDownloadTo

    Microsoft.PowerShell.Archive\Expand-Archive `
      -Force `
      -Path $BaProtoDownloadTo `
      -DestinationPath $BaProtoHomeBinPath
  
    if (!($NoModifyPath)) {
      $PathParts = [System.Environment]::GetEnvironmentVariable("PATH", "User").Trim(";") -Split ";"
      $PathParts = $PathParts.where{ $_ -ne $BaProtoHomeBinPath }
      $PathParts = $PathParts + $BaProtoHomeBinPath

      Write-Host "info: updating 'PATH' environment variable: ${BaProtoHomeBinPath}"

      [System.Environment]::SetEnvironmentVariable("PATH", $($PathParts -Join ";"), "User")
    }

    Write-Host "info: sucessfully installed executables:`n"
    Write-Host "  baproto.exe: $(Join-Path -Path $BaProtoHomeBinPath -ChildPath "baproto.exe")"
  }
  catch {
    Write-Host "error: failed to install 'baproto': ${_}"
  }
  finally {
    Write-Host "`ninfo: cleaning up downloads: ${BaProtoTempFolder}"

    Remove-Item -Recurse $BaProtoTempFolder
  }
}

Install
