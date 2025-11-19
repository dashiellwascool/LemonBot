using System;

namespace LemonBot.Models;

public class Config
{
    public static Config? Instance = null;
    public static readonly int CurrentVersion = 1;

    public int Version { get; set; } = CurrentVersion;
    public string DiscordApiKey { get; set; } = "";

    public Squawk Squawking { get; set; } = new Squawk();
    public partial class Squawk {
        public bool Enabled { get; set; } = false;
        public int MinSquawkTime { get; set; } = 0;
        public int MaxSquawkTime { get; set; } = 0;
        public double FuckYouResponseChance { get; set; } = 0;
        public double RandomResponseChance { get; set; } = 0;
        public int ResponseCooldown { get; set; } = 0;

        public List<ulong> RandomSquawkChannels { get; set; } = new();
    }
}
