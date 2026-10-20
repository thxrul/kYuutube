param(
    [Parameter(Mandatory = $true)][int]$RootProcessId,
    [ValidateRange(5, 300)][int]$Seconds = 30,
    [string]$OutputPath = '.\windows-spike-metrics.json'
)
$ErrorActionPreference = 'Stop'
$rootProcess = Get-Process -Id $RootProcessId
$rootStarted = $rootProcess.StartTime.ToUniversalTime().Ticks
$clock = [Diagnostics.Stopwatch]::StartNew()
$samples = [Collections.Generic.List[object]]::new()
$previous = @{}
$previousAt = 0.0
$cpuSeconds = 0.0
$intervalSeconds = 0.0
$status = 'completed'
while ($clock.Elapsed.TotalSeconds -lt $Seconds) {
    $rootProcess = Get-Process -Id $RootProcessId -ErrorAction SilentlyContinue
    if (-not $rootProcess -or $rootProcess.StartTime.ToUniversalTime().Ticks -ne $rootStarted) {
        $status = 'root_exited'
        break
    }
    # Query identity/ancestry only; never collect command lines or environments.
    $allProcesses = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name
    $tree = [Collections.Generic.HashSet[int]]::new()
    [void]$tree.Add($RootProcessId)
    do {
        $added = $false
        foreach ($item in $allProcesses) {
            if ($tree.Contains([int]$item.ParentProcessId) -and $tree.Add([int]$item.ProcessId)) {
                $added = $true
            }
        }
    } while ($added)
    $now = $clock.Elapsed.TotalSeconds
    $rss = 0L
    $deltaCpu = 0.0
    $entries = [Collections.Generic.List[object]]::new()
    $unreadable = [Collections.Generic.List[int]]::new()
    foreach ($childId in $tree) {
        $child = Get-Process -Id $childId -ErrorAction SilentlyContinue
        if (-not $child) { continue }
        try {
            $identity = "$childId/$($child.StartTime.ToUniversalTime().Ticks)"
            $total = $child.TotalProcessorTime.TotalSeconds
            if ($previous.ContainsKey($identity)) {
                $deltaCpu += [Math]::Max(0, $total - $previous[$identity])
            }
            $previous[$identity] = $total
            $rss += $child.WorkingSet64
            $entries.Add([ordered]@{ pid = $childId; name = $child.ProcessName; rss_mib = [Math]::Round($child.WorkingSet64 / 1MB, 2) })
        } catch {
            if ($childId -eq $RootProcessId) { throw }
            if (Get-Process -Id $childId -ErrorAction SilentlyContinue) { $unreadable.Add($childId) }
        }
    }
    $percentage = $null
    if ($samples.Count -gt 0) {
        $duration = [Math]::Max(0.001, $now - $previousAt)
        $percentage = 100 * $deltaCpu / $duration
        $cpuSeconds += $deltaCpu
        $intervalSeconds += $duration
    }
    $samples.Add([ordered]@{
        seconds = [Math]::Round($now, 3)
        rss_sum_mib = [Math]::Round($rss / 1MB, 2)
        one_core_cpu_percent = $percentage
        processes = @($entries.ToArray())
        unreadable_process_ids = @($unreadable.ToArray())
    })
    $previousAt = $now
    Start-Sleep -Seconds 1
}
if ($samples.Count -eq 0) { throw 'No process-tree samples were collected' }
$incomplete = @($samples | Where-Object { $_.unreadable_process_ids.Count -gt 0 }).Count
if ($status -eq 'completed' -and $incomplete -gt 0) { $status = 'incomplete' }
$rssValues = @($samples | ForEach-Object { $_.rss_sum_mib } | Sort-Object)
$middle = [int][Math]::Floor($rssValues.Count / 2)
$median = $rssValues[$middle]
if ($rssValues.Count % 2 -eq 0) { $median = ($median + $rssValues[$middle - 1]) / 2 }
$result = [ordered]@{
    status = $status
    incomplete_sample_count = $incomplete
    measurement = 'Windows process-tree working-set sum (RSS); shared pages can be counted more than once. CPU is percent of one core. Not PSS.'
    duration_seconds = [Math]::Round($clock.Elapsed.TotalSeconds, 3)
    rss_median_mib = $median
    rss_max_mib = ($rssValues | Measure-Object -Maximum).Maximum
    average_one_core_cpu_percent = [Math]::Round(100 * $cpuSeconds / [Math]::Max(0.001, $intervalSeconds), 2)
    max_process_count = ($samples | ForEach-Object { $_.processes.Count } | Measure-Object -Maximum).Maximum
    samples = @($samples.ToArray())
}
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding utf8
Write-Output "Saved process-tree measurements to $OutputPath"
if ($status -ne 'completed') { throw "Measurement did not complete reliably: $status" }
