# Staff Request Bot

This is an incredibly basic bot that I created as a means to introduce myself to Rust. It allows you to create a requests board for handling user requests.

### Building

The bot can be built using the `cargo build` command. If a release build is desired, you can use `cargo build --release`. The build artifact will be written within the `release` or `debug` directories in the `target` directory.

### Configuration

The bot will read its configuration from the path specified in the `STAFFBOT_CFG` environment variable OR `/usr/local/etc/staffbot.yaml` if `STAFFBOT_CFG` is unset.

The configuration is in YAML format. It should be formed like so: 

```YAML
token: "DISCORDTOKENHERE"
mongo_uri: "mongodb://127.0.0.1:27017/"
bot_prefix: "~"
mongo_database: "staff-requests"
```

You can create a token in the Discord developer portal if you don't already have one. The bot needs the `Message Content` intent, the `Server Members` intent, and the `Presence Update` intent (the last two are not strictly necessary, but are required for the help command to display correctly per the framework). 

### Usage

Given `~` as the bot prefix, a new request board can be configured like so (where # prefixed names are channel mentions and @ prefixed names are role mentions):

~setupBoard #requests #archive @Staff

Users will write requests in #requests and the bot will react to the messages with a checkmark and an X. @Staff can press one of the two (adding their own react) to take action on the request, at which point it will be deleted from #requests and sent to #archive.

A request board can be removed like so if and when it is no longer desired:

~removeBoard #requests

#### Missing Features

- Attachments are not archived when a message is moved to #archive
- There is minimal logging currently

### License

This package is released under the terms of the MIT license. Please see LICENSE.txt for more information.