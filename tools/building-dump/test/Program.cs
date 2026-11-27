using System;
using System.Collections.Generic;
using System.Linq;
using BuildingDumpMod;

namespace TestHarness
{
    // Fake stand-ins mirroring the real game's BuildingData shape.
    public class FakeVector2
    {
        public float x;
        public float y;
        public FakeVector2(float x, float y) { this.x = x; this.y = y; }
    }

    public class FakeBuildingMaterialEntry
    {
        public string item;
        public int quantity;
        public bool addToStorageAfterConstruction;
        public FakeBuildingMaterialEntry(string item, int qty, bool after)
        { this.item = item; this.quantity = qty; this.addToStorageAfterConstruction = after; }
    }

    public class BuildingData
    {
        public string identifier;
        public List<string> prerequisiteIdentifiers;
        public List<string> lowerTierIdentifiers;
        public bool enforceMaxAllowed;
        public int maxAllowed;
        public int weightToCatchFire = 1;
        public FakeVector2 gridSize;
        public int defaultBuilders;
        public int maxBuilders;
        public int workRequiredToConstruct;
        public int goldRequiredToRelocate;
        public int workRequiredToDeconstruct;
        public List<FakeBuildingMaterialEntry> buildingMaterials;
        public string buildGroup;
        public bool subjectToReserves = true;
        public float itemLifetimeModifier = 1f;
        public int defaultWorkers = 1;
        public int maxWorkers = 2;
        public string uiBuildCategory;
        public bool highlightSimilarBuildingsOnSelect = true;
        public string buildsiteClearingMode;
        public string placeableGridType;
        public float clearDetailsBorderWidth = 2f;
        public string notes;

        public BuildingData(string id, string cat, float w, float h, int workers, int maxW,
                            int work, int reloc, int decon, int maxAllowedValue, bool enforce,
                            List<string> prereqs, List<FakeBuildingMaterialEntry> materials,
                            string buildGroupValue, string notesValue)
        {
            identifier = id; uiBuildCategory = cat;
            gridSize = new FakeVector2(w, h);
            defaultWorkers = workers; maxWorkers = maxW;
            workRequiredToConstruct = work; goldRequiredToRelocate = reloc; workRequiredToDeconstruct = decon;
            maxAllowed = maxAllowedValue; enforceMaxAllowed = enforce;
            prerequisiteIdentifiers = prereqs;
            buildingMaterials = materials;
            buildGroup = buildGroupValue; notes = notesValue;
            lowerTierIdentifiers = new List<string>();
            clearDetailsBorderWidth = 2f;
        }
    }

    public class FakeBuildingSetupData
    {
        public List<BuildingData> buildingData;
        public FakeBuildingSetupData(List<BuildingData> data) { buildingData = data; }
    }

    public static class GlobalAssets
    {
        private static FakeBuildingSetupData _setup;
        public static FakeBuildingSetupData buildingSetupData
        {
            get
            {
                if (_setup == null)
                {
                    _setup = new FakeBuildingSetupData(new List<BuildingData>
                    {
                        new BuildingData("Academy", "AMENITIES_AND_SERVICES", 5f, 4f, 3, 5,
                            100, 0, 60, 0, false,
                            new List<string> { "TownCenter_Tier2", "School" },
                            new List<FakeBuildingMaterialEntry> {
                                new FakeBuildingMaterialEntry("ItemPlanks", 50, false),
                                new FakeBuildingMaterialEntry("ItemStone", 120, false),
                                new FakeBuildingMaterialEntry("ItemGoldIngot", 500, true),
                            },
                            "BUILDING", "teaching"),
                        new BuildingData("Hospital", "AMENITIES_AND_SERVICES", 6f, 6f, 2, 4,
                            150, 25, 80, 2, true,
                            new List<string> { "Academy" },
                            new List<FakeBuildingMaterialEntry> {
                                new FakeBuildingMaterialEntry("ItemPlanks", 100, false),
                                new FakeBuildingMaterialEntry("ItemStone", 50, false),
                            },
                            "BUILDING", "health"),
                        new BuildingData("TownCenter_Tier2", "ECONOMY", 8f, 8f, 4, 6,
                            300, 0, 200, 1, true,
                            new List<string> { "TownCenter_Tier1" },
                            new List<FakeBuildingMaterialEntry> {
                                new FakeBuildingMaterialEntry("ItemPlanks", 200, false),
                                new FakeBuildingMaterialEntry("ItemStone", 100, false),
                                new FakeBuildingMaterialEntry("ItemGoldIngot", 200, true),
                            },
                            "BUILDING", "nexus"),
                    });
                }
                return _setup;
            }
        }
    }

    public static class Program
    {
        public static int Main()
        {
            var root = DumpLogic.BuildDump();
            var json = root.ToString(Newtonsoft.Json.Formatting.Indented);
            Console.WriteLine(json);

            var total = (int)root["total"];
            var buildings = root["buildings"] as Newtonsoft.Json.Linq.JArray;
            Console.WriteLine($"--- total={total}, registry={root["registrySource"]}");

            int ok = 0;
            foreach (var b in buildings)
            {
                if ((string)b["identifier"] == "Academy")
                {
                    var prereqs = (Newtonsoft.Json.Linq.JArray)b["prerequisites"];
                    var mats = (Newtonsoft.Json.Linq.JObject)b["materials"];
                    var gridW = b["gridW"];
                    if (prereqs != null && prereqs.Count == 2 && (string)prereqs[0] == "TownCenter_Tier2" && (string)prereqs[1] == "School") ok++;
                    if (mats != null && mats["ItemPlanks"] != null && mats["ItemStone"] != null && mats["ItemGoldIngot"] != null) ok++;
                    if (gridW != null && (float)gridW == 5f && (float)b["gridH"] == 4f) ok++;
                }
            }
            Console.WriteLine($"checks passed: {ok}/3");
            return ok == 3 ? 0 : 1;
        }
    }
}
