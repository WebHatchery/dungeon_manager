<#
.SYNOPSIS
    Headless screenshot harness for Dungeon Manager (Deep Dominion).

.DESCRIPTION
    Thin wrapper around the shared macroquad-toolkit capture script. Builds the
    debug exe and drives it through the env-var capture hook
    (DUNGEON_MANAGER_CAPTURE_*) provided by macroquad_toolkit::capture in
    src/main.rs.

    Scenes:
      mainmenu     - the boot main menu
      gameplay     - a fresh dungeon behind the mission intro overlay. The
                     overlay freezes the simulation by design, so this shows
                     the briefing screen, NOT the game running.
      simulation   - the same dungeon with the intro dismissed and dig orders
                     seeded, so imps work, heroes spawn and objectives advance.
                     Use this to verify anything about the running game.
      wave         - "simulation" with the first hero wave pulled forward from
                     its authored 600s (36,000 frames) to 2s, so combat is
                     reachable in a capture. Needs ~900 frames to show a fight.
                     Note the seeded dig orders open the dungeon up, so this is
                     not a fair read on real wave-1 difficulty.
      raid         - "wave" plus a seeded army ordered at the hero base. The
                     keeper cannot assault it unaided: it sits behind solid
                     rock and creatures do not dig, so the raiders follow the
                     corridor the heroes tunnel in through. Reaches a mission
                     victory around 6600 frames.
      defeat       - the campaign defeat screen (retry / return buttons)
      campaign_end - the final-victory screen with the campaign epilogue
      skirmish, settings, missionselect - the other menu screens

    To see engine tracing during a capture, set DUNGEON_MANAGER_LOG (a
    comma-separated tag list, or "all") and drive the exe directly rather than
    through this wrapper - the shared script does not forward extra env vars.

.EXAMPLE
    ./scripts/capture_ui.ps1
    ./scripts/capture_ui.ps1 -Scenes simulation -Frames 600
    ./scripts/capture_ui.ps1 -Frames 60 -SkipBuild
#>
param(
    [string[]]$Scenes = @("mainmenu", "gameplay", "simulation"),
    [int]$Frames = 150,
    [string]$OutputDir = "docs\verification",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$gameDir = Split-Path -Parent $PSScriptRoot
$shared = Join-Path (Split-Path -Parent $gameDir) "macroquad-toolkit\scripts\capture_ui.ps1"

& $shared -GameDir $gameDir -Scenes $Scenes -Frames $Frames -OutputDir $OutputDir -SkipBuild:$SkipBuild
