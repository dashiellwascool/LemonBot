use std::sync::Arc;

use regex::{Regex, RegexBuilder};
use serenity::prelude::TypeMapKey;

pub struct TTSReplacements {
    pub replacements: [(Regex, &'static str); 3],
    pub emoji_capture: Regex,
    pub unicode_emoji: Regex,
}

impl TTSReplacements {
    pub fn new() -> anyhow::Result<Self> {
        Ok(TTSReplacements {
            replacements: [
                (
                    RegexBuilder::new(r"```(.*?)```")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |code block| ",
                ),
                (
                    RegexBuilder::new(r"`(.*?)`")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |code block|",
                ),
                (
                    RegexBuilder::new(r"\|\|(.*?)\|\|")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |spoilers| ",
                ),
            ],
            emoji_capture: Regex::new(r"<(a?):(.*?):\d+>")?,
            unicode_emoji: Regex::new(r"\p{Emoji}")?,
        })
    }

    pub(super) fn process_string(&self, text: &mut String) {
        self.run_replacements(text);
        self.process_discord_emojis(text);
        self.process_unicode_emojis(text);
    }

    fn run_replacements(&self, text: &mut String) {
        for (regex, str) in &self.replacements {
            *text = regex.replace_all(text, *str).into();
        }
    }

    fn process_discord_emojis(&self, text: &mut String) {
        *text = self
            .emoji_capture
            .replace_all(text, |capture: &regex::Captures<'_>| {
                let is_animated = capture
                    .get(1)
                    .expect("There are three capture groups")
                    .as_str();
                let emoji_name = capture
                    .get(2)
                    .expect("There are three capture groups")
                    .as_str();

                let emoji_type = if is_animated.is_empty() {
                    "emoji"
                } else {
                    "animated emoji"
                };

                format!(" {emoji_name} {emoji_type} ")
            })
            .into();
    }

    fn process_unicode_emojis(&self, text: &mut String) {
        *text = self
            .unicode_emoji
            .replace_all(text, |capture: &regex::Captures<'_>| {
                let emoji = capture.get(0).expect("there is one group").as_str();

                if let Some(emoji) = emojis::get(emoji) {
                    format!(" {} emoji ", emoji.name())
                } else {
                    emoji.to_string()
                }
            })
            .into();
    }
}

impl TypeMapKey for TTSReplacements {
    type Value = Arc<TTSReplacements>;
}
