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
$transitions = @($runtime | ForEach-Object {
    if ($_ -match '^# layer-changed from=(\d+) to=(\d+) elapsed_us=(\d+)') {
        [PSCustomObject]@{ From=[int]$Matches[1]; To=[int]$Matches[2]; Epoch=([long]$Matches[3]+$runtimeEpoch) }
    }
})
$letters = @{ '001E'='A'; '0030'='B'; '002E'='C' }
$held = @{}
$counts = @{}
$perLayer = @{}
$legacy = @{}
$alt = 0
$native01 = 0
$unmatched = 0
foreach ($row in $rows) {
    $parts = $row -split "`t",3
    if ($parts.Count -ne 3 -or $parts[0] -notmatch '^\d+$') { continue }
    $epoch = [long]$parts[0]+$observerEpoch
    if ($epoch -lt $runtimeEpoch -or $epoch -ge $stopEpoch) { continue }
    if ($parts[1] -match '^legacy ') {
        if (-not $legacy.ContainsKey($parts[1])) { $legacy[$parts[1]]=0 }; $legacy[$parts[1]]++
        continue
    }
    if ($parts[2] -notmatch 'VID_1532&PID_022B') { continue }
    if ($parts[1] -notmatch '^keyboard scan=([0-9A-F]{4}) flags=([0-9A-F]{4}) vk=([0-9A-F]{4})$') { continue }
    $scan = $Matches[1]
    $flags = [Convert]::ToUInt16($Matches[2],16)
    if ($scan -eq '0038' -and ($flags -band 6) -eq 0) { $alt++ }
    if ($scan -eq '0002' -and ($flags -band 6) -eq 0) { $native01++ }
    if (-not $letters.ContainsKey($scan) -or ($flags -band 6)) { continue }
    $letter = $letters[$scan]
    if (-not $counts.ContainsKey($letter)) { $counts[$letter]=@{ Presses=0; Repeats=0; Releases=0 } }
    if ($flags -band 1) {
        $counts[$letter].Releases++
        if ($held.ContainsKey($letter)) { $held.Remove($letter) } else { $unmatched++ }
    } else {
        if ($held.ContainsKey($letter)) { $counts[$letter].Repeats++ } else { $counts[$letter].Presses++ }
        $held[$letter]=$true
        $layer = 0
        foreach ($transition in $transitions) { if ($transition.Epoch -le $epoch) { $layer=$transition.To } }
        $label = "layer=$layer letter=$letter"
        if (-not $perLayer.ContainsKey($label)) { $perLayer[$label]=0 }; $perLayer[$label]++
    }
}
[ordered]@{
    Directory=(Resolve-Path -LiteralPath $Directory).Path
    TransitionSequence=@($transitions | ForEach-Object { "$($_.From)->$($_.To)" })
    LetterEvents=$counts; LetterDownEventsByLayer=$perLayer
    HeldLettersAtEnd=@($held.Keys); UnmatchedLetterReleases=$unmatched
    NativeThumbAltEventsDuringMapping=$alt; Native01EventsDuringMapping=$native01
    OrdinaryWindowsMessagesDuringMapping=$legacy
    Note='Layer association uses recorded transitions. Stop boundary approximated from completion file timestamp; letter counts include keypad and mapped wheel input.'
} | ConvertTo-Json -Depth 6
