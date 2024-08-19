# beer-bot

Reminder to celebrate a (possibly) long day with a beer 🍺.

## Description

A Slack Bot to announce it is time to enjoy a beer/< insert alcohol of choice>/< insert non-alcoholic beverage of
choice>.

## Build

### Default build
This is linked to your standard glibc stuff.

Nice and straight forward:

```shell
cargo build --release
```

Resulting in the binary `./target/release/beer-bot`.

### Using Musl for better compatability
You can instead use Musl libraries so that you're not tied to running the binary on the system it was compiled on.

#### Install the musl package
 * On Debian based systems:

```shell
sudo apt install musl-tools
```

 * On Arch based systems:

```shell
sudo pacman -S musl
```
#### Find the target arch
 * Pick an arch from the output:

```shell
rustup target list
```
#### Install the target arch
 * Repeat with each arch required:
```shell
rustup target add x86_64-unknown-linux-musl
```

### Build the target arch
 * Repeat with each arch required:
```shell
cargo build --target=x86_64-unknown-linux-musl --release
```

You will find the compiled target at:

```shell
./target/$arch/release/beer-bot
```

### Features

| Feature  | Quick Explanation                       | Enabled by Default |
|----------|-----------------------------------------|--------------------|
| commands | Enable slash commands using Socket Mode | ☑                  |
| syslog   | Output to syslog                        | ☐                  |

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

By default, Beer-bot listens for the following command(s):

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

beer-bot is configured by combining a config file and environment variables, where environment variables take precedence
over the config file.
All the options need to be specified.

### Options

| Key          | Meaning                                                              |
|--------------|----------------------------------------------------------------------|
| token        | Slack bot oAuth token - Requires `chat:write` scope                  |
| socket_token | Slack SocketMode token - Only required if `commands` feature enabled |
| crons        | List of cron expressions with a seconds column prepended.            |
| channel_id   | Either the channel name without the `#` or the ID in channel details |
| messages     | List of messages to randomly pick from for announcements             |
| log          | Log level directives                                                 |

The `log` option's structure is defined [here](https://docs.rs/env_logger/0.11.5/env_logger/#enabling-logging).

#### Logging

All logging has one of the following levels:

  * off
  * error
  * warn
  * info
  * debug
  * trace

By default, all logging is disabled since the default max log level to `off`.
By setting the level lower, any logging at the given log level or above in the above list are
outputted.

Some examples of valid log options:

* `info`: enables all info logging and above logging for the whole bot.
* `debug`: enables all debug logging and above for the whole bot.
This does include libraries used by beer-bot, so can get quite spammy.
* `warn,beer_bot=debug`: enables warn logging for the whole bot, except for logging specifically
from the bot which has debug and above logging.

### Environment Variables

Environment variables are the same as the config file keys, but in `SCREAMING_SNAKE_CASE` and prefixed with `BEERBOT_`.
Lists like `messages` are seperated by `¬`. (Needed a symbol that isn't likely to be in the messages).

#### Examples

```shell
BEERBOT_TOKEN="xo..."
BEERBOT_SOCKET_TOKEN="xapp..."
BEERBOT_CRONS="0 0 17 * * mon-thu *¬0 0 12 * * fri *"
BEERBOT_CHANNEL_ID="beer-bot"
BEERBOT_MESSAGES="Lets Go¬Its time to party"
BEERBOT_LOG="info"
```

### Config File

| Platform | Location                                                                         |
|----------|----------------------------------------------------------------------------------|
| Linux    | `$XDG_CONFIG_HOME/beerbot/beer-bot.toml` or `$HOME/.config/beerbot/beerbot.toml` |
| macOS    | `$HOME/Library/Application Support/com.beerbot.beerbot/beerbot.toml`             |
| Windows  | `{FOLDERID_LocalAppData}\\com\\beerbot\\beerbot\\config\\beerbot.toml`           |

#### Example

```toml
token = "xo..."
socket_token = "xapp..."
crons = ["0 0 17 * * mon-fri *","0 0 12 * * fri *"]
channel_id = "beer-bot"
messages = ["It's that time again", "LETS GO"]
log = "info"
```

License: MIT
