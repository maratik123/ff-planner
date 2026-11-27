# building-dump

A [MelonLoader](https://github.com/LavaGang/MelonLoader) mod for the Mono build of
*Farthest Frontier* that dumps the game's building definitions to JSON on startup.

## What it does

On game start the mod scans the loaded assemblies for the game's building data type
(`BuildingData`) and its static registries (e.g. `allBuildings`), serializes each
building to JSON, and writes it to `MelonLoader/UserData/building_dump.json`
next to the game (the working directory is the game root).

Output shape:

```json
{
  "source": "Assembly-CSharp.dll",
  "generatedAt": "...",
  "registrySource": "RegistryClass.allBuildingsRO",
  "total": 123,
  "buildings": [
    {
      "identifier": "Academy",
      "category": "AMENITIES_AND_SERVICES",
      "gridW": 5,
      "gridH": 4,
      "labor": 100,
      "relocateGold": 0,
      "deconstruct": 60,
      "desirabilityBonus": 0,
      "desirabilityRadius": 1,
      "dropoffEnabled": false,
      "allowMultiple": false,
      "tag": "Academy",
      "radii": { "entertainmentRadius": 50 },
      "prerequisites": ["TownCenter_Tier2", "School"],
      "materials": { "ItemPlanks": 50, "ItemStone": 120, "ItemGoldIngot": 500 },
      "extra": { "someOtherField": "..." }
    }
  ]
}
```

Curated fields are best-effort heuristic matches on well-known names; the
`extra` object carries everything else (including raw fields the curated matchers
did not claim) so nothing is lost even when the game's field names differ.

## Building

Requires .NET SDK (8+ for the harness; the mod itself targets net48).

```sh
dotnet build -p:GamePath="/path/to/Farthest Frontier (Mono)"
```

Copy the built `BuildingDumpMod.dll` into `MelonLoader/Mods/` inside the Mono build.

Then launch the game. On startup it writes `MelonLoader/UserData/building_dump.json`.

## Harness (offline check)

`test/TestHarness.csproj` exercises `DumpLogic` against a fake building registry
(no game install needed):

```sh
dotnet run --project test/TestHarness.csproj
```

The harness is intentionally **not** part of the mod build.
