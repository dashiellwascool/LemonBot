CREATE TABLE TTSUsers(
    discord_id BIGINT NOT NULL UNIQUE,
    nick TEXT,
    model TEXT,
    speaker TEXT
)
