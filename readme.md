# s3-discord

A discord bot that uses [s3s](https://github.com/frozenpandaman/s3s) to post your Splatoon 3 battles to discord

## Features

- Map, mode, result, and the score for each team
- Kills, assists, deaths, specials, and points for all players
- More detailed player info through a selection menu on the message
- Upload battle results and have them formatted

## Demo

Join [my server](https://discord.gg/GNGR5GQz) and send one of the [demo files](demo-files) in #general

Or

Watch the [Demo Video](https://youtu.be/y-eNDS-uO9g)

## How to run it yourself

These instructions only work on Linux (and might be able to be adapted for MacOS)

Download [s3s](https://github.com/frozenpandaman/s3s) and run it once to create the config file then press ctrl+c to quit

Download the appimage for the [latest build of nxapi](https://gitlab.fancy.org.uk/samuel/nxapi/-/jobs/artifacts/main/browse/app?job=build-app)\
Extract the appimage by running `Nintendo Switch Online-1.6.1.AppImage --appimage-extract`\
In the created folder run `./nxapi nso auth` and follow the insructions given by nxapi to connect your Nintendo account. Then run `./nxapi util update-s3s-token /path/to/s3s/config.txt`, replacing /path/to/s3s with wherever you downloaded s3s. This last command needs to be ran every few hours to update the bot's splatnet3 token.

In the s3s folder create an export folder and a results folder inside of that. Copy config.json.template to config.json and set results_dir to the results folder your created.

Create a Discord bot (I don't remember how, look it up) and put the bot's token in discord_token in the config.json

Invite the bot to a server (look it up). Turn on developer mode in discord settings->Advanced. Copy the channel id that you want the bot to send the messages to and put it between the square brackets in updates_channel_id in config.json

Run the bot with `cargo run -r` then, in another terminal tab, export battles by running `python s3s.py -o` (It takes a couple minutes for s3s to export battles)

s3s needs to be reran every time you want to upload new battles and nxapi needs to be reran about once a day to get new tokens for s3s

## Notes

To allow people to upload their battle logs set "recive_messages" in config.json to true and enable Message Content Intent on discord devleloper dashboard