param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$rows = Get-Content -LiteralPath (Join-Path $Directory 'observer.tsv')
if (-not ($rows | Select-String '^# complete ')) { throw 'Observer is still recording' }
$runtimePath = Join-Path $Directory 'runtime.log'
$runtime = Get-Content -LiteralPath $runtimePath
if (-not ($runtime | Select-String '^# runtime stopped')) { throw 'Runtime completion is not recorded' }
$observerEpoch = [long](($rows | Select-String '^# start_epoch_us=').Line -replace '^# start_epoch_us=','')
$runtimeEpoch = [long](($runtime | Select-Object -First 1) -replace '.*start_epoch_us=','')
# Runtime writes its completion last. File timestamp is an approximate stop boundary.
$stopEpoch = [long](((Get-Item -LiteralPath $runtimePath).LastWriteTimeUtc.Ticks - [DateTime]::new(1970,1,1,0,0,0,[DateTimeKind]::Utc).Ticks) / 10)
$letters = @{ '001E' = 'A'; '0030' = 'B'; '002E' = 'C'; '0020' = 'D' }
$held = @{}
$counts = @{}
$pairTime = @{}
$lastTime = $runtimeEpoch
$unmatchedUps = 0
$nativeLeaks = 0
$restoredArrows = 0
$otherArrows = 0
$numberedEvents = 0
$numberedEventsWithDpadHeld = 0
$nativeHeld = @{}
$legacy = @{}
foreach ($row in $rows) {
    $parts = $row -split "`t",3
    if ($parts.Count -ne 3 -or $parts[0] -notmatch '^\d+$') { continue }
    $epoch = [long]$parts[0] + $observerEpoch
    $active = $epoch -ge $runtimeEpoch -and $epoch -lt $stopEpoch
    $target = $parts[2] -match 'VID_1532&PID_022B'
    if ($parts[1] -match '^keyboard scan=([0-9A-F]{4}) flags=([0-9A-F]{4}) vk=([0-9A-F]{4})$') {
        $scan = $Matches[1]
        $flags = [Convert]::ToUInt16($Matches[2],16)
        $arrow = $scan -in @('0048','0050','004B','004D') -and ($flags -band 6) -eq 2
        if ($arrow -and $target) {
            if ($active) { $nativeLeaks++ } else { $restoredArrows++ }
            if (-not $active) {
                if ($flags -band 1) { $nativeHeld.Remove($scan) } else { $nativeHeld[$scan] = $true }
            }
        }
        if ($arrow -and $parts[2] -eq 'other-keyboard' -and $active) { $otherArrows++ }
        if ($target -and $active -and $scan -eq '0002') {
            $numberedEvents++
            if ($held.Count -gt 0) { $numberedEventsWithDpadHeld++ }
        }
        if ($target -and $active -and $letters.ContainsKey($scan) -and ($flags -band 6) -eq 0) {
            if ($held.Count -eq 2) {
                $pair = ($held.Keys | Sort-Object) -join '+'
                if (-not $pairTime.ContainsKey($pair)) { $pairTime[$pair] = [long]0 }
                $pairTime[$pair] += $epoch - $lastTime
            }
            $lastTime = $epoch
            $letter = $letters[$scan]
            if (-not $counts.ContainsKey($letter)) { $counts[$letter] = @{ Presses = 0; Repeats = 0; Releases = 0 } }
            if ($flags -band 1) {
                $counts[$letter].Releases++
                if ($held.ContainsKey($letter)) { $held.Remove($letter) } else { $unmatchedUps++ }
            } else {
                if ($held.ContainsKey($letter)) { $counts[$letter].Repeats++ } else { $counts[$letter].Presses++ }
                $held[$letter] = $true
            }
        }
    } elseif ($parts[1] -match '^legacy vk=([0-9A-F]{4}) message=([0-9A-F]{4})$' -and $active) {
        $signature = $parts[1]
        if (-not $legacy.ContainsKey($signature)) { $legacy[$signature] = 0 }
        $legacy[$signature]++
    }
}
[ordered]@{
    Directory = (Resolve-Path -LiteralPath $Directory).Path
    LetterEvents = $counts
    UnmatchedLetterReleases = $unmatchedUps
    HeldLettersAtEnd = @($held.Keys)
    NativeTargetArrowEventsDuringMapping = $nativeLeaks
    NativeTargetArrowEventsOutsideMapping = $restoredArrows
    OtherKeyboardArrowEventsDuringMapping = $otherArrows
    Numbered01EventsDuringMapping = $numberedEvents
    Numbered01EventsWhileDpadHeld = $numberedEventsWithDpadHeld
    NativeArrowKeysHeldAtObserverEnd = @($nativeHeld.Keys)
    OverlappingLetterPairsMicroseconds = $pairTime
    OrdinaryWindowsMessagesDuringMapping = $legacy
    Note = 'Stop boundary approximated from runtime completion file timestamp. Overlap totals describe observed two-component holds, not physical orientation.'
} | ConvertTo-Json -Depth 6
