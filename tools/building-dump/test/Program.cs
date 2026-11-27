using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using BuildingDumpMod;

namespace TestHarness
{
    // Fake stand-ins mimicking the game's building data shape.
    public enum BuildingTagPlaceholder { Academy, Hospital }

    public class BuildingData
    {
        public string buildingIdentifier;
        public string buildingCategory;
        public int gridWidth;
        public int gridHeight;
        public int labor;
        public int relocateGold;
        public int deconstruct;
        public int desirabilityBonus;
        public int desirabilityRadius;
        public bool dropoffEnabled;
        public bool allowMultiple;
        public string tag;
        public Dictionary<string, int> radii;
        public string prerequisites;
        public string materials;
        public string notes;

        public BuildingData(string id, string cat, int w, int h, int labor, int reloc, int decon,
                                int bonus, int radius, bool dropoff, bool multi, string tag,
                                Dictionary<string, int> radii, string prereqs, string materials, string notes)
        {
            buildingIdentifier = id;
            buildingCategory = cat;
            gridWidth = w; gridHeight = h;
            this.labor = labor;
            this.relocateGold = reloc;
            this.deconstruct = decon;
            this.desirabilityBonus = bonus;
            this.desirabilityRadius = radius;
            this.dropoffEnabled = dropoff;
            this.allowMultiple = multi;
            this.tag = tag;
            this.radii = radii;
            this.prerequisites = prereqs;
            this.materials = materials;
            this.notes = notes;
        }
    }

    public static class BuildingRegistry
    {
        public static List<BuildingData> allBuildingsRO = new List<BuildingData>
        {
            new BuildingData("Academy", "AMENITIES_AND_SERVICES", 5, 4, 100, 0, 60, 0, 1,
                false, false, "Academy",
                new Dictionary<string, int> { { "entertainmentRadius", 50 } },
                "TownCenter_Tier2|School",
                "50 ItemPlanks, 120 ItemStone, 500 ItemGoldIngot",
                "teaching"),
            new BuildingData("Hospital", "AMENITIES_AND_SERVICES", 6, 6, 150, 25, 80, 10, 2,
                true, false, "Hospital",
                new Dictionary<string, int> { { "entertainmentRadius", 75 }, { "healingRadius", 30 } },
                "Academy",
                "100 ItemPlanks, 50 ItemStone",
                "health"),
            new BuildingData("TownCenter_Tier2", "ECONOMY", 8, 8, 300, 0, 200, 0, 1,
                false, true, "TownCenter",
                new Dictionary<string, int> { { "economicRadius", 100 } },
                "TownCenter_Tier1",
                "200 ItemPlanks, 100 ItemStone, 200 ItemGoldIngot",
                "nexus")
        };
    }

    public static class Program
    {
        public static int Main()
        {
            // Assembly-CSharp won't exist; DumpLogic falls back to scanning all assemblies.
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
                    var radii = (Newtonsoft.Json.Linq.JObject)b["radii"];
                    Console.WriteLine($"Academy check: prereqs=[{string.Join(",", prereqs.Select(p => (string)p))}] mats={mats.ToString()} radii={radii.ToString()}");
                    if (prereqs.Count == 2 && (string)prereqs[0] == "TownCenter_Tier2" && (string)prereqs[1] == "School") ok++;
                    if (mats["ItemPlanks"] != null && mats["ItemStone"] != null && mats["ItemGoldIngot"] != null) ok++;
                    if (radii["entertainmentRadius"] != null) ok++;
                }
            }
            Console.WriteLine($"checks passed: {ok}/3");
            return ok == 3 ? 0 : 1;
        }
    }
}
