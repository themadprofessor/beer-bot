# beer-bot

Reminder to celebrate a (possibly) long day with a beer 🍺.

## Description
A Slack Bot to announce it is time to enjoy a beer/< insert alcohol of choice>/< insert non-alcoholic beverage of choice>.

## Build

Nice and straight forward:
```shell
cargo build --release
```
Resulting in the binary `./target/release/beer-bot`.


### Features

| Feature  | Quick Explanation                       | Enabled by Default |
|----------|-----------------------------------------|--------------------|
| syslog   | Output to syslog                        | ☐                  |
| commands | Enable slash commands using Socket Mode | ☑                  |

Features are additive.
So to have Beer Bot output to Syslog and not enable slash commands, all default features must first be disabled:

```shell
cargo build --release --no-default-features --features "syslog"
```

#### Syslog Feature

By default, Beer Bot output to stdout, but this can be changed to utilise syslog.
The logging levels are mapped to syslog severities according to the following table:

| Log Level | Syslog Severity |
|-----------|-----------------|
| ERROR     | Error           |
| WARN      | Warning         |
| INFO      | Notice          |
| DEBUG     | Info            |
| TRACE     | Debug           |

#### Commands Feature

With this feature disabled, beer-bot spends almost all of its time sleeping.
Only waking when any of the crons trigger.
Therefore, if slash commands are not needed, it's recommended to disable them.

Rather than require beer-bot to act as a HTTP server and what that entails to receive slash
commands, it utilises [Socket Mode](https://api.slack.com/apis/socket-mode).
Enabling Socket Mode in Slack's App Config will generate an "App Level Token" which beer-bot
use to establish the web socket between it and Slack.
This token *must* be passed to beer-bot using the `socket_token` [option](#options).

Once Socket Mode is enabled, a [Slash Command](https://api.slack.com/interactivity/slash-commands)
can be created without having to specify an endpoint.

Beer-bot listens for the following commands:
* `when-can-i-drink`

### Docker
First create a config file called `config.toml`.
See [Options](#options).

```shell
docker compose up
```
Build beer-bot, create a minimal image with beer-bot and run the image.
This file is included in the `docker-compose.yml` as a secret.

## Usage

Nice and straight forward:
```shell
beer-bot
```

beer-bot is configured by combining a config file and environment variables, where environment variables take precedence over the config file.
All the options need to be specified.

### Options
| Key          | Meaning                                                              |
| ------------ | -------------------------------------------------------------------- |
| token        | Slack bot oAuth token - Requires `chat:write` scope                  |
| socket_token | Slack SocketMode token - Only required if `commands` feature enabled |
| crons        | List of cron expressions with a seconds column prepended.            |
| channel_id   | Either the channel name without the `#` or the ID in channel details |
| messages     | List of messages to randomly pick from for announcements             |

### Environment Variables

Environment variables are the same as the config file keys, but in `SCREAMING_SNAKE_CASE` and prefixed with `BEERBOT_`.
Lists like `messages` are seperated by `¬`. (Needed a symbol that isn't likely to be in the messages).

#### Examples
```shell
BEERBOT_TOKEN="xo..."
BEERBOT_CHANNEL_ID="beer-bot"
BEERBOT_MESSAGES="Lets Go¬Its time to party"
```

### Config File
|Platform | Location                                                                         |
| ------- | -------------------------------------------------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME/beerbot/beer-bot.toml` or `$HOME/.config/beerbot/beerbot.toml` |
| macOS   | `$HOME/Library/Application Support/com.beerbot.beerbot/beerbot.toml`             |
| Windows | `{FOLDERID_LocalAppData}\\com\\beerbot\\beerbot\\config\\beerbot.toml`            |

#### Example
```toml
token = "xo..."
crons = ["0 0 17 * * mon-fri *"]
channel_id = "beer-bot"
messages = ["It's that time again", "LETS GO"]
```

License: MIT
