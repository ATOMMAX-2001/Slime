$sub = Register-ObjectEvent -InputObject ([Console]) -EventName CancelKeyPress -Action {
    $EventArgs.Cancel = $true
    Set-Location ..
    exit
}

try
{
    Set-Location web
    maturin develop
    if ($LASTEXITCODE -ne 0)
    {
        throw "Failed to build extension"
    }
    Set-Location ../slimeweb/
    uv run python -Xgil=0 -m test.test
} catch
{
    Write-Error "Run Failed: $($_.Exception.Message)"
    Set-Location ..
}
