using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
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
        public static JObject BuildDump()
        {
            var buildingsAsm = AppDomain.CurrentDomain.GetAssemblies()
                .FirstOrDefault(a => a.GetName().Name == "Assembly-CSharp");
            var buildingType = buildingsAsm == null
                ? null
                : AllTypes(buildingsAsm).FirstOrDefault(t => t.Name == "BuildingData");
            if (buildingType == null)
            {
                buildingType = AppDomain.CurrentDomain.GetAssemblies()
                    .SelectMany(AllTypes)
                    .FirstOrDefault(t => t.Name == "BuildingData");
            }

            var buildings = new List<JObject>();
            var registrySource = "";

            if (buildingType != null)
            {
                foreach (var source in FindRegistries(buildingType))
                {
                    var count = 0;
                    foreach (var item in source.Value)
                    {
                        if (item == null) continue;
                        buildings.Add(BuildEntry(item, buildingType));
                        count++;
                    }
                    if (count > 0)
                    {
                        registrySource = source.Name;
                        break;
                    }
                    registrySource = source.Name;
                }
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

        private static JObject BuildEntry(object building, Type buildingType)
        {
            var entry = new JObject();
            var fields = CollectFields(buildingType, building, out var extra);

            SetString(entry, "identifier", FindString(fields, "identifier", "id", "key", "buildingName", "buildingIdentifier"));
            SetString(entry, "category", FindString(fields, "buildingCategory", "category", "buildingCategoryName"));
            SetNumber(entry, "gridW", FindNumber(fields, "gridWidth", "gridW", "width", "sizeX"));
            SetNumber(entry, "gridH", FindNumber(fields, "gridHeight", "gridH", "height", "sizeY"));
            SetNumber(entry, "labor", FindNumber(fields, "labor", "laborCost", "laborersRequired", "laborNeeded", "requiredLabor"));
            SetNumber(entry, "relocateGold", FindNumber(fields, "relocateGold", "relocationGold", "relocateCost", "relocate"));
            SetNumber(entry, "deconstruct", FindNumber(fields, "deconstruct", "deconstructCost", "destructionCost", "demolish"));
            SetNumber(entry, "desirabilityBonus", FindNumber(fields, "desirabilityBonus", "additionalDesirability", "bonusDesirability"));
            SetNumber(entry, "desirabilityRadius", FindNumber(fields, "desirabilityRadius"));
            SetBool(entry, "dropoffEnabled", FindNumber(fields, "dropoffEnabled", "allowDropoff", "dropOffEnabled", "allowDropOff"));
            SetBool(entry, "allowMultiple", FindNumber(fields, "allowMultiple", "allowMultipleBuildings", "multipleAllowed"));
            SetString(entry, "tag", FindString(fields, "tag", "buildingTag", "buildingTagName"));

            var radii = FindRadiusMap(fields);
            if (radii.Properties().Count() > 0) entry["radii"] = radii;
            var prereqs = FindPrereqArray(fields);
            if (prereqs.Count > 0) entry["prerequisites"] = prereqs;
            var mats = FindMaterialMap(fields);
            if (mats.Properties().Count() > 0) entry["materials"] = mats;

            if (extra != null && extra.Properties().Count() > 0)
                entry["extra"] = extra;

            return entry;
        }

        private static void SetString(JObject entry, string key, string value)
        {
            if (!string.IsNullOrEmpty(value)) entry[key] = value;
        }

        private static void SetNumber(JObject entry, string key, JToken value)
        {
            if (value != null) entry[key] = value;
        }

        private static void SetBool(JObject entry, string key, JToken value)
        {
            if (value == null) return;
            var s = value.ToString();
            if (bool.TryParse(s, out var b)) entry[key] = b;
            else entry[key] = value;
        }

        // ---- reflection helpers ----

        private static IEnumerable<Type> AllTypes(Assembly a)
        {
            try { return a.GetTypes(); }
            catch { return new Type[0]; }
        }

        private class RegistryInfo
        {
            public string Name;
            public IEnumerable<object> Value;
        }

        private static IEnumerable<RegistryInfo> FindRegistries(Type buildingType)
        {
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
            foreach (var type in AllTypes(asm))
            {
                IEnumerable<PropertyInfo> props;
                IEnumerable<FieldInfo> fields;
                try { props = type.GetProperties(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic); }
                catch { continue; }
                try { fields = type.GetFields(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic); }
                catch { continue; }

                foreach (var prop in props)
                {
                    var val = SafeGetValue(prop);
                    if (!IsEnumerableOfElements<object>(val, buildingType)) continue;
                    yield return new RegistryInfo
                    {
                        Name = $"{type.Name}.{prop.Name}",
                        Value = val as IEnumerable<object> ?? (val as IEnumerable).Cast<object>()
                    };
                }
                foreach (var field in fields)
                {
                    var val = SafeGetValue(field);
                    if (!IsEnumerableOfElements<object>(val, buildingType)) continue;
                    yield return new RegistryInfo
                    {
                        Name = $"{type.Name}.{field.Name}",
                        Value = val as IEnumerable<object> ?? (val as IEnumerable).Cast<object>()
                    };
                }
            }
        }

        private static object SafeGetValue(PropertyInfo prop)
        {
            try { return prop.GetValue(null, null); }
            catch { return null; }
        }

        private static object SafeGetValue(FieldInfo field)
        {
            try { return field.GetValue(null); }
            catch { return null; }
        }

        private static bool IsEnumerableOfElements<T>(object val, Type buildingType)
        {
            if (val == null) return false;
            var en = val as IEnumerable;
            if (en == null) return false;
            foreach (var item in en)
            {
                if (item == null) continue;
                if (buildingType == null) return true;
                if (buildingType.IsInstanceOfType(item)) return true;
                return false;
            }
            return buildingType == null;
        }

        private static JObject CollectFields(Type t, object obj, out JObject extra)
        {
            extra = new JObject();
            var dict = new JObject();
            foreach (var f in t.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic))
            {
                object val;
                try { val = f.GetValue(obj); }
                catch { continue; }
                if (val == null) continue;
                var conv = ConvertValue(val, 0);
                if (conv == null) continue;
                dict[f.Name] = conv;
                extra[f.Name] = conv;
            }
            foreach (var p in t.GetProperties(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic))
            {
                if (!p.CanRead) continue;
                object val;
                try { val = p.GetValue(obj, null); }
                catch { continue; }
                if (val == null) continue;
                var conv = ConvertValue(val, 0);
                if (conv == null) continue;
                if (!dict.ContainsKey(p.Name)) dict[p.Name] = conv;
            }
            // remove curated names from extra to reduce duplication noise
            var curated = new HashSet<string>(new[]
            {
                "buildingIdentifier","identifier","id","key","buildingName",
                "buildingCategory","category","buildingCategoryName",
                "gridWidth","gridW","width","sizeX","gridHeight","gridH","height","sizeY",
                "labor","laborCost","laborersRequired","laborNeeded","requiredLabor",
                "relocateGold","relocationGold","relocateCost","relocate",
                "deconstruct","deconstructCost","destructionCost","demolish",
                "desirabilityBonus","additionalDesirability","bonusDesirability",
                "desirabilityRadius","desirability",
                "dropoffEnabled","allowDropoff","dropOffEnabled","allowDropOff",
                "allowMultiple","allowMultipleBuildings","multipleAllowed",
                "tag","buildingTag","buildingTagName",
                "radii","radiuslist","prerequisites","prerequisite","materials","items","resources","materialList"
            }, StringComparer.OrdinalIgnoreCase);

            var iter = new List<string>(extra.Properties().Select(p => p.Name));
            foreach (var k in iter)
            {
                if (curated.Contains(k)) extra.Remove(k);
            }
            return dict;
        }

        private static JToken ConvertValue(object val, int depth)
        {
            if (val == null) return null;
            if (depth > 3) return new JValue("<complex>");

            var type = val.GetType();

            if (val is string s) return new JValue(s);
            if (val is bool b) return new JValue(b);
            if (val is byte || val is sbyte || val is short || val is ushort || val is int || val is uint || val is long || val is ulong)
                return new JValue(Convert.ToInt64(val));
            if (val is float) return new JValue((double)(float)val);
            if (val is double) return new JValue((double)val);
            if (val is decimal) return new JValue((double)(decimal)val);
            if (val is DateTime dt) return new JValue(dt.ToString("o"));

            if (type.IsEnum) return new JValue(val.ToString());

            if (type.Namespace != null && type.Namespace.StartsWith("UnityEngine"))
                return new JValue(val.ToString());

            if (val is IDictionary dict)
            {
                var jobj = new JObject();
                foreach (DictionaryEntry de in dict)
                {
                    var key = de.Key?.ToString() ?? "null";
                    var c = ConvertValue(de.Value, depth + 1);
                    if (c != null) jobj[key] = c;
                }
                return jobj;
            }

            var en = val as IEnumerable;
            if (en != null && !(val is string))
            {
                var arr = new JArray();
                foreach (var item in en)
                {
                    var c = ConvertValue(item, depth + 1);
                    if (c != null) arr.Add(c);
                }
                return arr;
            }

            if (type.IsValueType || type.GetProperties().Length > 0 || type.GetFields().Length > 0)
            {
                var jobj = new JObject();
                foreach (var f in type.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic))
                {
                    try
                    {
                        var fv = f.GetValue(val);
                        var c = ConvertValue(fv, depth + 1);
                        if (c != null) jobj[f.Name] = c;
                    }
                    catch { }
                }
                if (jobj.Properties().Count() == 0) return new JValue(val.ToString());
                return jobj;
            }

            return new JValue(val.ToString());
        }

        private static JArray FindPrereqArray(JObject fields)
        {
            foreach (var prop in fields.Properties())
            {
                var name = prop.Name.ToLowerInvariant();
                if (name.Contains("prereq") || name.Contains("prerequisite") || name.Contains("requires"))
                {
                    if (prop.Value is JArray a) return a;
                    if (prop.Value is JObject o)
                    {
                        var nested = o["list"] ?? o["items"] ?? o["prerequisites"];
                        if (nested is JArray arr) return arr;
                    }
                    if (prop.Value is JValue jv)
                    {
                        var s = jv.ToString();
                        if (string.IsNullOrWhiteSpace(s)) return new JArray();
                        var parts = s.Split(new[] { '|', ',' }, StringSplitOptions.RemoveEmptyEntries).Select(p => p.Trim()).Where(p => p.Length > 0);
                        return new JArray(parts);
                    }
                }
            }
            return new JArray();
        }

        private static JObject FindRadiusMap(JObject fields)
        {
            var result = new JObject();
            foreach (var prop in fields.Properties())
            {
                var name = prop.Name.ToLowerInvariant();
                if (name.Contains("radius") && !name.Contains("desirability"))
                {
                    if (prop.Value is JObject o)
                    {
                        foreach (var sub in o.Properties())
                            result[sub.Name] = sub.Value;
                    }
                    else if (prop.Value is JArray a)
                    {
                        for (int i = 0; i < a.Count; i++)
                            result[$"{prop.Name}[{i}]"] = a[i];
                    }
                    else
                        result[prop.Name] = prop.Value;
                }
            }
            foreach (var prop in fields.Properties())
            {
                var n = prop.Name.ToLowerInvariant();
                if (n == "radii" || n == "radiuslist")
                {
                    if (prop.Value is JObject o)
                    {
                        foreach (var sub in o.Properties())
                            if (!result.ContainsKey(sub.Name)) result[sub.Name] = sub.Value;
                    }
                    else if (prop.Value is JArray a)
                    {
                        for (int i = 0; i < a.Count; i++)
                        {
                            if (a[i] is JObject o2)
                            {
                                var key = o2["name"] ?? o2["type"] ?? o2["id"];
                                var val = o2["value"] ?? o2["radius"] ?? o2["amount"];
                                if (key != null && val != null) result[key.ToString()] = val;
                            }
                        }
                    }
                }
            }
            return result;
        }

        private static JObject FindMaterialMap(JObject fields)
        {
            var result = new JObject();
            foreach (var prop in fields.Properties())
            {
                var name = prop.Name.ToLowerInvariant();
                if (!name.Contains("material") && !name.Contains("item") && !name.Contains("resource")) continue;

                var v = prop.Value;
                if (v is JArray a)
                {
                    foreach (var item in a)
                    {
                        if (item is JObject o)
                        {
                            var id = o["item"] ?? o["id"] ?? o["material"] ?? o["name"] ?? o["itemName"] ?? o["resource"];
                            var qty = o["count"] ?? o["qty"] ?? o["quantity"] ?? o["amount"] ?? o["value"];
                            if (id != null && qty != null)
                                result[id.ToString()] = qty;
                            else if (id != null && o["count"] == null)
                                result[id.ToString()] = id;
                        }
                        else if (item is JValue jv)
                        {
                            var s = jv.ToString().Trim();
                            foreach (var part in s.Split(new[] { ',', ';' }, StringSplitOptions.RemoveEmptyEntries))
                            {
                                var tokens = part.Trim().Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
                                if (tokens.Length >= 2 && int.TryParse(tokens[0], out var qty))
                                    result[string.Join(" ", tokens.Skip(1))] = qty;
                                else if (tokens.Length == 1)
                                    result[tokens[0]] = 1;
                            }
                        }
                    }
                }
                else if (v is JObject o)
                {
                    foreach (var sub in o.Properties())
                    {
                        // nested dict: { "ItemPlanks": 50, ... }
                        if (sub.Value is JValue) result[sub.Name] = sub.Value;
                        else
                        {
                            var qty = sub.Value["count"] ?? sub.Value["qty"] ?? sub.Value["quantity"];
                            if (qty != null) result[sub.Name] = qty;
                        }
                    }
                }
                else if (v is JValue jv)
                {
                    var s = jv.ToString().Trim();
                    if (s.Length > 0)
                    {
                        // "50 ItemPlanks, 120 ItemStone" style
                        foreach (var part in s.Split(new[] { ',', ';' }, StringSplitOptions.RemoveEmptyEntries))
                        {
                            var tokens = part.Trim().Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
                            if (tokens.Length >= 2 && int.TryParse(tokens[0], out var qty))
                                result[string.Join(" ", tokens.Skip(1))] = qty;
                            else if (tokens.Length == 1)
                                result[tokens[0]] = 1;
                        }
                    }
                }
            }
            return result;
        }

        private static string FindString(JObject fields, params string[] names)
        {
            foreach (var n in names)
            {
                foreach (var prop in fields.Properties())
                {
                    if (prop.Name.Equals(n, StringComparison.OrdinalIgnoreCase) && prop.Value != null)
                        return prop.Value.ToString();
                }
            }
            foreach (var name in names)
            {
                foreach (var prop in fields.Properties())
                {
                    if (prop.Name.ToLowerInvariant().Contains(name.ToLowerInvariant()) && prop.Value is JValue v)
                        return v.ToString();
                }
            }
            return null;
        }

        private static JToken FindNumber(JObject fields, params string[] names)
        {
            foreach (var n in names)
            {
                foreach (var prop in fields.Properties())
                {
                    if (prop.Name.Equals(n, StringComparison.OrdinalIgnoreCase))
                        return prop.Value;
                }
            }
            foreach (var name in names)
            {
                foreach (var prop in fields.Properties())
                {
                    if (prop.Name.ToLowerInvariant().Contains(name.ToLowerInvariant()))
                        return prop.Value;
                }
            }
            return null;
        }
    }
}
