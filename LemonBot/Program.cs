using Discord;
using Discord.WebSocket;
using LemonBot;
using LemonBot.Features;
using LemonBot.Models;
using LemonBot.Utilities;

_ = new Logger();

Console.WriteLine("LemonBot version 2.1.0");
Console.WriteLine("Made with love by bloofirephoenix");

if (!ConfigHelpers.InitializeConfig("config.json", out Config.Instance))
{
    Logger.Warning("config does not exist");
    Console.WriteLine("plz setup config.json plz i beg of you");
    return;
}

if (Config.Instance!.Version != Config.CurrentVersion) {
    Logger.Warning("config is out of date! :(");
    Logger.Warning("dont worry im updating config.json");
    Config.Instance.Version = Config.CurrentVersion;
    ConfigHelpers.SaveConfig(Config.Instance, "config.json");
}

var client = new DiscordSocketClient(new DiscordSocketConfig()
{
    GatewayIntents = GatewayIntents.GuildMessages | GatewayIntents.Guilds
});

client.Log += Logger.DiscordLog;

await client.LoginAsync(TokenType.Bot, Config.Instance.DiscordApiKey);
await client.StartAsync();

client.Ready += () => {
    Console.WriteLine("Bot Ready :O");

    // enable features
    if (Config.Instance.Squawking.Enabled)
        new Squawk(client).Start();

    return Task.CompletedTask;
};

await TaskManager.Run();