using System;
using System.IO;
using MelonLoader;

namespace BuildingDumpMod
{
    public class BuildingDumpMod : MelonMod
    {
        public override void OnApplicationStart()
        {
            MelonLogger.Msg("BuildingDumpMod: starting");
            try
            {
                var dump = DumpLogic.BuildDump();
                var outDir = Path.Combine(Directory.GetCurrentDirectory(), "UserData");
                Directory.CreateDirectory(outDir);
                var path = Path.Combine(outDir, "building_dump.json");
                File.WriteAllText(path, dump.ToString(Newtonsoft.Json.Formatting.Indented));
                MelonLogger.Msg($"BuildingDumpMod: wrote {path}");
            }
            catch (Exception ex)
            {
                MelonLogger.Error($"BuildingDumpMod: failed: {ex}");
            }
        }
    }
}
