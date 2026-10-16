param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
$rows = Get-Content -LiteralPath (Join-Path $Directory 'observer.tsv')
if (-not ($rows | Select-String '^# complete ')) { throw 'Observer is still recording' }
$runtimePath = Join-Path $Directory 'runtime.log'
$runtime = Get-Content -LiteralPath $runtimePath
if (-not ($runtime | Select-String '^# runtime stopped')) { throw 'Runtime completion is not recorded' }
$observerEpoch = [long](($rows | Select-String '^# start_epoch_us=').Line -replace '^# start_epoch_us=','')
$runtimeEpoch = [long](($runtime | Select-Object -First 1) -replace '.*start_epoch_us=','')
$stopEpoch = [long](((Get-Item -LiteralPath $runtimePath).LastWriteTimeUtc.Ticks - [DateTime]::new(1970,1,1,0,0,0,[DateTimeKind]::Utc).Ticks) / 10)
$letters = @{ '001E' = 'A'; '0030' = 'B'; '002E' = 'C' }
$held = @{}
$counts = @{}
$native = @{}
$other = @{}
$leaks = 0
$unmatched = 0
$scrollWhileMiddleHeld = 0
$legacy = @{}
foreach ($row in $rows) {
    $parts = $row -split "`t",3
    if ($parts.Count -ne 3 -or $parts[0] -notmatch '^\d+$') { continue }
    $epoch = [long]$parts[0] + $observerEpoch
    $active = $epoch -ge $runtimeEpoch -and $epoch -lt $stopEpoch
    $target = $parts[2] -match 'VID_1532&PID_022B'
    if ($parts[1] -match '^keyboard scan=([0-9A-F]{4}) flags=([0-9A-F]{4}) vk=([0-9A-F]{4})$' -and $target -and $active) {
        $scan = $Matches[1]
        $flags = [Convert]::ToUInt16($Matches[2],16)
        if (-not $letters.ContainsKey($scan) -or ($flags -band 6)) { continue }
        $letter = $letters[$scan]
        if (-not $counts.ContainsKey($letter)) { $counts[$letter] = @{ Presses=0; Repeats=0; Releases=0 } }
        if ($flags -band 1) {
            $counts[$letter].Releases++
            if ($held.ContainsKey($letter)) { $held.Remove($letter) } else { $unmatched++ }
        } else {
            if ($held.ContainsKey($letter)) { $counts[$letter].Repeats++ } else { $counts[$letter].Presses++ }
            if ($letter -in @('A','B') -and $held.ContainsKey('C')) { $scrollWhileMiddleHeld++ }
            $held[$letter] = $true
        }
    } elseif ($parts[1] -match '^mouse flags=([0-9A-F]{4}) buttons=([0-9A-F]{4}) wheel=(-?\d+)') {
        $buttons = [Convert]::ToUInt16($Matches[2],16)
        $delta = [int]$Matches[3]
        if (($buttons -band 0x430) -eq 0) { continue }
        $signature = "buttons=$($Matches[2]) wheel=$delta"
        if ($target -and $active) { $leaks++ }
        if ($target -and -not $active) {
            if (-not $native.ContainsKey($signature)) { $native[$signature]=0 }; $native[$signature]++
        }
        if ($parts[2] -eq 'other-mouse' -and $active) {
            if (-not $other.ContainsKey($signature)) { $other[$signature]=0 }; $other[$signature]++
        }
    } elseif ($parts[1] -match '^legacy vk=' -and $active) {
        if (-not $legacy.ContainsKey($parts[1])) { $legacy[$parts[1]]=0 }; $legacy[$parts[1]]++
    }
}
[ordered]@{
    Directory = (Resolve-Path -LiteralPath $Directory).Path
    LetterEvents = $counts; HeldLettersAtEnd = @($held.Keys); UnmatchedLetterReleases = $unmatched
    OriginalTargetWheelOrMiddleEventsDuringMapping = $leaks
    ScrollTapsWhileMappedMiddleHeld = $scrollWhileMiddleHeld
    OtherMouseEventsDuringMapping = $other
    NativeTargetMouseEventsOutsideMapping = $native
    OrdinaryWindowsMessagesDuringMapping = $legacy
    Note = 'Active boundary approximated from runtime completion log timestamp. A/B are wheel taps; C is the middle-click hold.'
} | ConvertTo-Json -Depth 6
