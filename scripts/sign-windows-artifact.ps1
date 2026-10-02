[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ArtifactPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-RequiredEnvironmentValue {
    param([Parameter(Mandatory = $true)][string]$Name)

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "Required release-signing environment variable '$Name' is not configured."
    }
    return $value
}

$resolvedArtifact = (Resolve-Path -LiteralPath $ArtifactPath -ErrorAction Stop).Path
if ([IO.Path]::GetExtension($resolvedArtifact) -notin @('.exe', '.msi')) {
    throw 'Azure Artifact Signing is restricted to SiteDatum .exe and .msi release artifacts.'
}

$endpoint = Get-RequiredEnvironmentValue 'AZURE_ARTIFACT_SIGNING_ENDPOINT'
$accountName = Get-RequiredEnvironmentValue 'AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME'
$certificateProfileName = Get-RequiredEnvironmentValue 'AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE_NAME'
[void](Get-RequiredEnvironmentValue 'AZURE_CLIENT_ID')
[void](Get-RequiredEnvironmentValue 'AZURE_TENANT_ID')

$clientSecret = [Environment]::GetEnvironmentVariable('AZURE_CLIENT_SECRET')
$federatedTokenFile = [Environment]::GetEnvironmentVariable('AZURE_FEDERATED_TOKEN_FILE')
if ([string]::IsNullOrWhiteSpace($clientSecret) -and [string]::IsNullOrWhiteSpace($federatedTokenFile)) {
    throw 'Azure signing authentication is missing. Configure a protected client secret or federated token file.'
}

$endpointUri = [Uri]$endpoint
if ($endpointUri.Scheme -ne 'https' -or $endpointUri.Host -notlike '*.codesigning.azure.net') {
    throw 'AZURE_ARTIFACT_SIGNING_ENDPOINT must be an HTTPS Azure codesigning endpoint.'
}

$clientTools = Join-Path ${env:ProgramFiles(x86)} 'Microsoft\ArtifactSigningClientTools\bin'
$dlibPath = Join-Path $clientTools 'Azure.CodeSigning.Dlib.dll'
$signToolPath = Join-Path $clientTools 'signtool.exe'
if (-not (Test-Path -LiteralPath $dlibPath -PathType Leaf)) {
    throw 'Azure Artifact Signing Client Tools are not installed or the Dlib path is unavailable.'
}
if (-not (Test-Path -LiteralPath $signToolPath -PathType Leaf)) {
    $signToolCommand = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -eq $signToolCommand) {
        throw 'A compatible Windows SDK SignTool was not found.'
    }
    $signToolPath = $signToolCommand.Source
}

$metadataPath = Join-Path ([IO.Path]::GetTempPath()) ("sitedatum-artifact-signing-{0}.json" -f [Guid]::NewGuid().ToString('N'))
try {
    @{
        Endpoint = $endpointUri.AbsoluteUri
        CodeSigningAccountName = $accountName
        CertificateProfileName = $certificateProfileName
    } | ConvertTo-Json | Set-Content -LiteralPath $metadataPath -Encoding utf8NoBOM

    & $signToolPath sign /v /fd SHA256 /tr 'https://timestamp.acs.microsoft.com' /td SHA256 /dlib $dlibPath /dmdf $metadataPath $resolvedArtifact
    if ($LASTEXITCODE -ne 0) {
        throw "Azure Artifact Signing failed with exit code $LASTEXITCODE."
    }

    & $signToolPath verify /v /pa $resolvedArtifact
    if ($LASTEXITCODE -ne 0) {
        throw "Authenticode verification failed with exit code $LASTEXITCODE."
    }
}
finally {
    if (Test-Path -LiteralPath $metadataPath) {
        Remove-Item -LiteralPath $metadataPath -Force
    }
}

