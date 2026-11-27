using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace BuildingDumpMod
{
    // Pure reflection-based dump logic, no MelonLoader dependency, so it can be
    // exercised from a console harness.
    public static class DumpLogic
    {
        // Curated scalar field mappings (instance field/property name -> JSON key).
        // Objects beyond these scalars (Vector2, lists, prefabs, enums) are handled
        // explicitly or fall into `extra`.
        private static readonly Dictionary<string, string> CuratedMap = new Dictionary<string, string>
        {
            { "identifier", "identifier" },
            { "uiBuildCategory", "category" },
            { "buildGroup", "buildGroup" },
            { "defaultBuilders", "defaultBuilders" },
            { "maxBuilders", "maxBuilders" },
            { "workRequiredToConstruct", "workRequiredToConstruct" },
            { "goldRequiredToRelocate", "goldRequiredToRelocate" },
            { "workRequiredToDeconstruct", "workRequiredToDeconstruct" },
            { "enforceMaxAllowed", "enforceMaxAllowed" },
            { "maxAllowed", "maxAllowed" },
            { "weightToCatchFire", "weightToCatchFire" },
            { "highlightSimilarBuildingsOnSelect", "highlightSimilarBuildingsOnSelect" },
            { "subjectToReserves", "subjectToReserves" },
            { "itemLifetimeModifier", "itemLifetimeModifier" },
            { "defaultWorkers", "defaultWorkers" },
            { "maxWorkers", "maxWorkers" },
            { "buildsiteClearingMode", "buildsiteClearingMode" },
            { "placeableGridType", "placeableGridType" },
            { "clearDetailsBorderWidth", "clearDetailsBorderWidth" },
            { "materialsListDirty", "materialsListDirty" },
        };

        // Members consumed by the curated sections; never appear in `extra`.
        private static readonly HashSet<string> ConsumedNames = new HashSet<string>
        {
            "gridSize", "prerequisiteIdentifiers", "buildingMaterials", "lowerTierIdentifiers", "notes"
        };

        public static JObject BuildDump()
        {
            var buildingType = ResolveBuildingType();
            var registry = ResolveRegistry(buildingType);
            var buildings = new List<JObject>();
            string registrySource = "";

            if (buildingType != null && registry.Item2 != null)
            {
                registrySource = registry.Item1;
                foreach (var item in registry.Item2)
                {
                    if (item == null) continue;
                    if (!buildingType.IsInstanceOfType(item)) continue;
                    buildings.Add(BuildEntry(item, buildingType));
                }
                if (buildings.Count == 0) registrySource = "";
            }

            return new JObject
            {
                ["source"] = "Assembly-CSharp.dll",
                ["generatedAt"] = DateTime.Now.ToString("o"),
                ["registrySource"] = registrySource,
                ["total"] = buildings.Count,
                ["buildings"] = new JArray(buildings)
            };
        }

        private static Type ResolveBuildingType()
        {
            return AppDomain.CurrentDomain.GetAssemblies()
                .SelectMany(a => AllTypes(a))
                .FirstOrDefault(t => t.Name == "BuildingData");
        }

        // Returns (registryName, enumerable of BuildingData)
        private static Tuple<string, IEnumerable> ResolveRegistry(Type buildingType)
        {
            // 1. Typed path: GlobalAssets.buildingSetupData.buildingData
            try
            {
                var assetsType = AppDomain.CurrentDomain.GetAssemblies()
                    .SelectMany(a => AllTypes(a))
                    .FirstOrDefault(t => t.Name == "GlobalAssets");
                if (assetsType != null)
                {
                    var prop = assetsType.GetProperty("buildingSetupData",
                        BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    var setup = prop?.GetValue(null, null);
                    if (setup != null)
                    {
                        var list = GetMemberValue(setup, "buildingData");
                        if (list is IEnumerable en)
                        {
                            return Tuple.Create("GlobalAssets.buildingSetupData.buildingData",
                                (IEnumerable)en);
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                MelonLog("ResolveRegistry (GlobalAssets path) failed: " + ex.Message);
            }

            // 2. Fallback: any static property/field whose value enumerates BuildingData
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
            {
                foreach (var type in AllTypes(asm))
                {
                    foreach (var mem in type.GetMembers(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
                    {
                        object val;
                        string name;
                        switch (mem)
                        {
                            case PropertyInfo p:
                                if (!p.CanRead || p.GetIndexParameters().Length > 0) continue;
                                try { val = p.GetValue(null, null); } catch { continue; }
                                name = p.Name;
                                break;
                            case FieldInfo f:
                                try { val = f.GetValue(null); } catch { continue; }
                                name = f.Name;
                                break;
                            default:
                                continue;
                        }
                        if (val is string) continue;
                        if (!(val is IEnumerable en2)) continue;
                        bool isBuildings = false;
                        try
                        {
                            foreach (var item in en2)
                            {
                                if (item != null && buildingType != null && buildingType.IsInstanceOfType(item))
                                { isBuildings = true; break; }
                            }
                        }
                        catch { continue; }
                        if (isBuildings)
                        {
                            return Tuple.Create($"{type.Name}.{name}", en2);
                        }
                    }
                }
            }
            return Tuple.Create<string, IEnumerable>("", null);
        }

        private static JObject BuildEntry(object building, Type buildingType)
        {
            var entry = new JObject();
            var extra = new JObject();
            var seen = new HashSet<string>();

            // Collect members: public instance fields + readable public properties (no indexers)
            var members = new List<Tuple<string, object>>();
            foreach (var f in buildingType.GetFields(BindingFlags.Public | BindingFlags.Instance))
            {
                object val;
                try { val = f.GetValue(building); } catch { continue; }
                members.Add(Tuple.Create(f.Name, val));
            }
            foreach (var p in buildingType.GetProperties(BindingFlags.Public | BindingFlags.Instance))
            {
                if (p.GetIndexParameters().Length > 0) continue;
                if (!p.CanRead) continue;
                object val;
                try { val = p.GetValue(building, null); } catch { continue; }
                if (members.Any(m => m.Item1 == p.Name)) continue; // field already collected
                members.Add(Tuple.Create(p.Name, val));
            }

            // ---- scalar curated / extra ----
            foreach (var m in members)
            {
                var name = m.Item1;
                var val = m.Item2;
                if (val is null) continue;
                if (val is string || val is bool || val is int || val is short || val is long ||
                    val is float || val is double || val is char || val is byte)
                {
                    if (CuratedMap.TryGetValue(name, out var key))
                    {
                        entry[key] = JToken.FromObject(val);
                        seen.Add(name);
                    }
                    else if (!ConsumedNames.Contains(name))
                    {
                        extra[name] = JToken.FromObject(val);
                        seen.Add(name);
                    }
                    continue;
                }
                // enum values: SerializeValue -> raw name
                if (val.GetType().IsEnum)
                {
                    if (CuratedMap.TryGetValue(name, out var ekey))
                    {
                        entry[ekey] = val.ToString();
                        seen.Add(name);
                    }
                    else if (!ConsumedNames.Contains(name))
                    {
                        extra[name] = val.ToString();
                        seen.Add(name);
                    }
                    continue;
                }
                // strings handled above; everything else (objects) ignored at this stage
            }

            // ---- gridSize: Vector2-like -> gridW/gridH ----
            var grid = GetMemberValue(building, "gridSize");
            if (grid != null)
            {
                var x = GetMemberValue(grid, "x");
                var y = GetMemberValue(grid, "y");
                if (x != null && y != null)
                {
                    entry["gridW"] = JToken.FromObject(x);
                    entry["gridH"] = JToken.FromObject(y);
                }
                seen.Add("gridSize");
                extra.Remove("gridSize");
            }

            // ---- prerequisites: List<string> etc. ----
            var prereq = GetMemberValue(building, "prerequisiteIdentifiers");
            if (prereq is IEnumerable enPrereq)
            {
                var list = new JArray();
                foreach (var s in enPrereq)
                {
                    if (s == null) continue;
                    var str = s.ToString();
                    if (!string.IsNullOrEmpty(str)) list.Add(str);
                }
                entry["prerequisites"] = list;
                seen.Add("prerequisiteIdentifiers");
                extra.Remove("prerequisiteIdentifiers");
            }

            // ---- materials: List<BuildingMaterialEntry> -> { item: qty } ----
            var materials = GetMemberValue(building, "buildingMaterials");
            if (materials is IEnumerable enMats)
            {
                var matMap = new JObject();
                var materialsAfter = new JObject();
                foreach (var item in enMats)
                {
                    if (item == null) continue;
                    var itemName = GetMemberValue(item, "item")?.ToString();
                    var quantity = GetMemberValue(item, "quantity");
                    var addAfter = GetMemberValue(item, "addToStorageAfterConstruction");
                    if (string.IsNullOrEmpty(itemName)) continue;
                    matMap[itemName] = quantity == null ? null : JToken.FromObject(quantity);
                    if (addAfter is bool b && b)
                        materialsAfter[itemName] = quantity == null ? null : JToken.FromObject(quantity);
                }
                entry["materials"] = matMap;
                if (materialsAfter.Count > 0)
                    entry["materialsAfterConstruction"] = materialsAfter;
                seen.Add("buildingMaterials");
                extra.Remove("buildingMaterials");
            }

            // ---- lowerTierIdentifiers: List<string> ----
            var lowerTier = GetMemberValue(building, "lowerTierIdentifiers");
            if (lowerTier is IEnumerable enLower)
            {
                var list = new JArray();
                foreach (var s in enLower)
                {
                    if (s == null) continue;
                    var str = s.ToString();
                    if (!string.IsNullOrEmpty(str)) list.Add(str);
                }
                if (list.Count > 0)
                {
                    entry["lowerTierIdentifiers"] = list;
                    seen.Add("lowerTierIdentifiers");
                    extra.Remove("lowerTierIdentifiers");
                }
            }

            if (extra.Count > 0)
                entry["extra"] = extra;

            return entry;
        }

        private static void MelonLog(string msg)
        {
            Console.WriteLine("[DumpLogic] " + msg);
        }

        private static object GetMemberValue(object obj, string name)
        {
            var t = obj.GetType();
            var prop = t.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            if (prop != null && prop.CanRead)
            {
                try { return prop.GetValue(obj, null); } catch { }
            }
            var field = t.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            if (field != null)
            {
                try { return field.GetValue(obj); } catch { }
            }
            return null;
        }

        private static IEnumerable<Type> AllTypes(Assembly asm)
        {
            Type[] types;
            try { types = asm.GetTypes(); }
            catch { yield break; }
            foreach (var t in types) yield return t;
        }
    }
}
