[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Path)
$ErrorActionPreference = 'Stop'
$rows = Get-Content -LiteralPath $Path
if (-not ($rows | Where-Object { $_ -match '^# complete ' })) { throw 'Capture is incomplete; wait until the recorder exits.' }
$held = @{}
$keys = [ordered]@{}
$mouse = @{}
$unmatchedUps = 0
$keyboardEvents = 0
$mouseEvents = 0
$hidEvents = 0
foreach ($row in $rows) {
    $parts = $row -split "`t", 3
    if ($parts.Count -ne 3) { continue }
    $detail = $parts[1]
    $source = $parts[2]
    if ($detail -match '^keyboard scan=([0-9A-F]{4}) flags=([0-9A-F]{4}) vk=([0-9A-F]{4})$') {
        $keyboardEvents++
        $scan = $Matches[1]
        $flags = [Convert]::ToUInt16($Matches[2], 16)
        $vk = $Matches[3]
        $identity = "$source|$scan|$($flags -band 6)"
        if (-not $keys.Contains($identity)) {
            $keys[$identity] = [ordered]@{ Scan = $scan; ExtendedFlags = ($flags -band 6); VirtualKey = $vk; Source = $source; Presses = 0; Repeats = 0; Releases = 0 }
        }
        if ($flags -band 1) {
            $keys[$identity].Releases++
            if ($held.ContainsKey($identity)) { $held.Remove($identity) } else { $unmatchedUps++ }
        } else {
            if ($held.ContainsKey($identity)) { $keys[$identity].Repeats++ } else { $keys[$identity].Presses++ }
            $held[$identity] = $true
        }
    } elseif ($detail -like 'mouse *') {
        $mouseEvents++
        if (-not $mouse.ContainsKey($detail)) { $mouse[$detail] = 0 }
        $mouse[$detail]++
    } elseif ($detail -like 'hid *') { $hidEvents++ }
}
[ordered]@{
    Trace = (Resolve-Path -LiteralPath $Path).Path
    KeyboardEvents = $keyboardEvents; MouseEvents = $mouseEvents; HidEvents = $hidEvents
    KeysInFirstObservedOrder = @($keys.Values)
    HeldKeysAtEnd = @($held.Keys); UnmatchedKeyReleases = $unmatchedUps
    MouseEventGroups = $mouse
    Note = 'Observed Windows events only; physical labels and rollover remain unverified. Keys held before capture may produce unmatched releases.'
} | ConvertTo-Json -Depth 8
