using Discord;
using Discord.WebSocket;
using LemonBot.Models;
using LemonBot.Utilities;

namespace LemonBot.Features;

public class Squawk(DiscordSocketClient client)
{
    private static bool _started = false;
    public void Start()
    {
        if (_started)
        {
            Logger.Warning("Attempted to start squawk after already starting squawk!!!!");
            return;
        }
        _started = true;
        Console.WriteLine("enabling squawk.");
        Console.WriteLine("warming up the vocal chords as we speak");
        client.MessageReceived += OnMessageReceived;
        TaskManager.RunTask(Run);
    }

    private async Task OnMessageReceived(SocketMessage message)
    {
        if (message.Author.Id == client.CurrentUser.Id)
            return;
        bool mentioned = false;
        foreach (var user in message.MentionedUsers)
        {
            if (user.Id == client.CurrentUser.Id)
            {
                mentioned = true;
                break;
            }
        }
        
        if (!mentioned)
        {
            foreach (var role in message.MentionedRoles)
            {
                if (role.Guild.CurrentUser.Roles.Contains(role))
                {
                    mentioned = true;
                    break;
                }
            }
        }

        mentioned = mentioned || message.MentionedEveryone;

        var reference = new MessageReference(message.Id);
        if (mentioned)
        {
            Console.WriteLine("Squawking because I was mentioned >:(");
            var sayFuckYou = Random.Shared.NextDouble() < Config.Instance!.Squawking.FuckYouResponseChance;
            await SendSquawk(message.Channel, reference, sayFuckYou);
        }
        else if (Config.Instance!.Squawking.RandomSquawkChannels.Contains(message.Channel.Id) && 
                 SaveData.Instance!.NoResponseUntil <= DateTime.Now.Ticks / TimeSpan.TicksPerMillisecond &&
                 Random.Shared.NextDouble() < Config.Instance.Squawking.RandomResponseChance)
        {
            Console.WriteLine("squawking because someone said something");
            SaveData.Instance.NoResponseUntil = DateTime.Now.Ticks / TimeSpan.TicksPerMillisecond 
                                                 + Config.Instance.Squawking.ResponseCooldown;
            ConfigHelpers.SaveConfig(SaveData.Instance, "data.json");

            var sayFuckYou = Random.Shared.NextDouble() < Config.Instance.Squawking.FuckYouResponseChance;
            await SendSquawk(message.Channel, reference, sayFuckYou);
        }
    }

    private async Task SendSquawk(ISocketMessageChannel channel, MessageReference? reference = null, bool fuckYou = false)
    {
        var message = fuckYou ? "fuk u" : "Squawk!";

        if (channel is ITextChannel)
        {
            var guidChannel = (ITextChannel) channel;
            var user = await guidChannel.Guild.GetCurrentUserAsync();
            if (!user.GetPermissions(guidChannel).SendMessages)
            {
                Console.WriteLine("aborting squawk because no permissions :(");
                return;
            }
        }

        await channel.SendMessageAsync(message, messageReference: reference, 
            flags: MessageFlags.SuppressNotification, allowedMentions: AllowedMentions.None);
    }

    private async void Run()
    {
        Console.WriteLine("enabling random squawk");
        while (true)
        {
            if ((Config.Instance!.Squawking.MinSquawkTime <= 0 && Config.Instance!.Squawking.MaxSquawkTime <= 0) || 
                Config.Instance!.Squawking.MaxSquawkTime < Config.Instance!.Squawking.MinSquawkTime)
            {
                Logger.Warning("random squawk times are invalid!");
                Console.WriteLine("disabling random squawk");
                return;
            }

            if (!ConfigHelpers.InitializeConfig("data.json", out SaveData.Instance)) 
            {
                Logger.Warning("save data could not be loaded");
            }

            var currentMilliseconds = DateTime.Now.Ticks / TimeSpan.TicksPerMillisecond;
            int waitTime;
            if (currentMilliseconds > SaveData.Instance!.NextSquawk)
            {
                waitTime = Random.Shared.Next(Config.Instance!.Squawking.MinSquawkTime, Config.Instance!.Squawking.MaxSquawkTime);
                SaveData.Instance.NextSquawk = currentMilliseconds + waitTime;
                ConfigHelpers.SaveConfig(SaveData.Instance, "data.json");
            } 
            else 
            {
                waitTime = (int) (SaveData.Instance.NextSquawk - currentMilliseconds);
            }

            var time = TimeSpan.FromMilliseconds(waitTime);
            
            Console.WriteLine($"squawking in {time:dd':'hh':'mm':'ss'.'FFF}");
            await Task.Delay(time);

            if (Config.Instance.Squawking.RandomSquawkChannels.Count <= 0) // man i really hope its not less than 0
            {
                Logger.Warning("no random squawk channels specified");
                Console.WriteLine("disabling random squawk");
                return;
            }

            Console.WriteLine("sending random squawk");
            
            var channelId = Config.Instance.Squawking.RandomSquawkChannels[Random.Shared.Next(0, 
                Config.Instance.Squawking.RandomSquawkChannels.Count)];
            var channel = (ISocketMessageChannel) await client.GetChannelAsync(channelId);
            await SendSquawk(channel);
        }
    }
}
