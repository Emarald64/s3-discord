use serenity::all::Interaction;
use tokio;
use notify::{self, Event, EventKind, Watcher, event};
use serde::Deserialize;
use tokio::time::Instant;
use std::env;
// use serde_json::Value;
// use std::io::Read;
use std::{collections::HashMap, path::PathBuf};
use std::time::Duration;
// use std::{fmt::Display};
// use std::str::FromStr;
use anyhow::{bail,anyhow};
use std::{sync::{mpsc,Arc,Mutex},fs::{File,self}};
use crate::{battle::*,stats::*};
// use std::{fs};

use serenity::prelude::*;
use serenity::all::*;
use serenity::{model::{id::ChannelId,application::ComponentInteractionDataKind}};

use serenity::async_trait;

mod battle;
mod stats;

// const S3S_RESULTS_DIR:&str="/home/agiller/.config/s3s/exports/results/";
// const SEND_CHANNEL_ID:ChannelId=ChannelId::new(1481734356832882852);
const CONFIG_PATH:&str="config.json";

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    //read config file
    let config:Config;
    {
        // let mut config_buf=String::default();
        let config_file=File::open(CONFIG_PATH).expect("config.json missing");
        // config_file.read_to_string(&mut config_buf)?;
        config=serde_json::from_reader(&config_file)?;
    }

    let results_path=PathBuf::from(&config.results_dir);

    let intents=if config.recive_messages{ GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT} else {GatewayIntents::GUILD_MESSAGES};
    //setup notify to check s3s results folder
    let (tx,rx)=mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&results_path, notify::RecursiveMode::NonRecursive)?;

    start_auto_update(config.s3s_path, config.nxapi_path, config.s3s_config_path,config.update_game_interval.unwrap_or(30),results_path.parent().map(|path|path.parent()).flatten().ok_or(anyhow!("invalid results path"))?.to_path_buf());

    //read saved stats
    let stats=Arc::new(Mutex::new(
            match get_tracked_stats()?{
                Some(stats)=>stats,
                None=>from_past_games(&config.results_dir, config.tracked_players.clone()).await?
            }
        )
    );

    // dbg!(&stats);

    //setup discord
    let mut client=Client::builder(config.discord_token, intents).event_handler(Handeler{results_path:results_path,update_channels:config.updates_channel_ids.clone(), stats:Arc::clone(&stats), tracked_players:config.tracked_players.clone()}).await?;
    let http= Arc::clone(&client.http);
    tokio::spawn(async move {let _=client.start().await;});
    loop{
        // wait for new log
        match rx.recv(){
            Ok(Ok(event))=>if let EventKind::Create(event::CreateKind::File)=event.kind{
                let path=event.paths[0].as_path();
                // wait for 1 sec after the file was created
                let wait=match fs::metadata(path){
                    Ok(metadata)=>match metadata.created(){
                        Ok(time)=>match time.elapsed(){
                            Ok(durr)=>Duration::from_secs(1).checked_sub(durr),
                            Err(_)=>Some(Duration::from_secs(1))
                        },
                        Err(_)=>Some(Duration::from_secs(1))
                    },
                    Err(_)=>Some(Duration::from_secs(1)),
                };
                // if the file is more than a second old, don't wait
                if let Some(wait)=wait{
                    tokio::time::sleep(wait).await;
                }
                println!("scanning file at {}",path.to_string_lossy());
                let file=File::open(path).expect("invalid file path");
                if let Ok(battle)=serde_json::from_reader(file){
                    println!("parsing battle");
                    match Battle::from_map(battle){
                        Ok(battle)=>{
                            if !config.excluded_modes.contains(&battle.mode) || !config.excluded_lobbies.contains(&battle.lobby){
                                println!("{}",&battle);
                                if let Ok(mut stats)=stats.lock(){
                                    add_game(&mut stats,&battle,&config.tracked_players);
                                }

                                // for player in &battle.our_players{
                                //     if config.tracked_players.contains(&player.name){
                                //         if let Some(player_stats)=stats.get_mut(&player.name){
                                //             player_stats
                                //         }
                                //     }
                                // }
                                // post log to discord
                                for channel_id in &config.updates_channel_ids{
                                    let _=channel_id.send_message(&http, battle.to_message(Some(path.file_name().expect("File path ends in ..").to_str().expect("string is not valid utf-8")))).await;
                                }
                            }
                        },
                        Err(err)=>{println!("{}",err);}
                    }
                }
            },
            Ok(Err(e))=>println!("watch error: {:?}", e),
            Err(e)=>bail!(e)
        };
    };
}


struct Handeler{
    results_path:PathBuf,
    update_channels:Vec<ChannelId>,
    stats:Arc<Mutex<HashMap<String,TotalPlayerStats>>>,
    tracked_players:Vec<String>
}

#[async_trait]
impl EventHandler for Handeler{
    async fn interaction_create(&self,ctx:Context,interaction:Interaction){
        // dbg!(&interaction);
        if let Interaction::Component(mut interaction)=interaction{
            let data=&interaction.data;
            if let ComponentInteractionDataKind::StringSelect{values:data_values}=&data.kind{
                println!("opening {}",&data.custom_id);
                let path=self.results_path.join(&data.custom_id);
                match File::open(&path){
                    Ok(file)=>{
                        if let Ok(battle)=serde_json::from_reader(file){
                            if let Ok(battle)=Battle::from_map(battle){
                                println!("getting data for {}",data_values[0]);
                                let _=if let Some(player)=battle.our_players.iter().chain(battle.their_players.iter()).find(|player|{player.name.clone()+&player.name_id==data_values[0]}){
                                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                        .content(format!("```{player}   Weapon:{}\n{}\n\nGear:\n{}\nPrimary Ability          Primary Ability          Primary Ability\n{}\n\nSecondary Abilities      Secondary Abilities      Secondary Abilities\n{}\n{}\n{}```",
                                            player.weapon,
                                            player.byname,
                                            player.gears.iter().fold(String::from(""),|acc,gear|{format!("{acc}{:25}",gear.name)}),
                                            player.gears.iter().fold(String::from(""),|acc,gear|{format!("{acc}{:25}",gear.primary_ability)}),
                                            player.gears.iter().fold(String::from(""),|acc,gear|{format!("{acc}{:25}",gear.display_secondary_ability(0))}),
                                            player.gears.iter().fold(String::from(""),|acc,gear|{format!("{acc}{:25}",gear.display_secondary_ability(1))}),
                                            player.gears.iter().fold(String::from(""),|acc,gear|{format!("{acc}{:25}",gear.display_secondary_ability(2))}),
                                        ))
                                        .ephemeral(true)
                                    )).await
                                }else{
                                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(format!("{} not found",data_values[0])))).await
                                };
                            }
                        }else{
                            let _=interaction.create_response(&ctx,CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Could not find battle"))).await;
                            let _=interaction.message.edit(ctx, EditMessage::new().components(vec![]));
                        }
                    },
                    Err(err)=>println!("Error opening {}: {}",path.to_string_lossy(),err)
                }
            }
        }
    }

    async fn message(&self,ctx:Context,new_message:Message){
        if self.update_channels.contains(&new_message.channel_id){
            // dbg!(&new_message.content);
            let channel_id=new_message.channel_id;
            if new_message.attachments.len()>0{
                // println!("message in correct channel");
                let typing=serenity::http::Typing::start(ctx.http.clone(), new_message.channel_id);
                for attachment in new_message.attachments{
                    let ctx_clone=(&ctx).clone();
                    tokio::spawn(async move {
                        let _=if attachment.filename.ends_with(".json") && attachment.size<=500_000{
                            if let Ok(content)=attachment.download().await{
                                if let Ok(battle_map) = serde_json::from_slice(content.as_slice()) {
                                    match Battle::from_map(battle_map){
                                        Ok(battle)=>{channel_id.send_message(&ctx_clone, battle.to_message(None)).await},
                                        Err(err)=>{channel_id.say(&ctx_clone, format!("failed to parse attchment: {} due to error: {}",attachment.filename,err)).await}
                                    }
                                }
                                else{
                                    channel_id.say(&ctx_clone, format!("failed to parse attachment: {}",attachment.filename)).await
                                }
                            }else{
                                channel_id.say(&ctx_clone, format!("failed to download attachment: {}",attachment.filename)).await
                            }
                        }else{
                            channel_id.say(&ctx_clone, format!("attachment: {} is too large or has wrong extention",attachment.filename)).await
                        };
                    });
                }
                typing.stop();
            }else if new_message.content.starts_with("/stats"){
                // println!("stats command");
                let message=match self.stats.lock(){
                    Ok(stats)=>{
                        Some(channel_id.say(ctx,
                        if let Some((_,name))=new_message.content.split_once(' ')
                        && let Some(player_stats)=stats.get(&name.to_uppercase()){
                            player_stats.to_string()
                        }else{
                            //list names
                            self.tracked_players.iter().fold(String::new(), |acc,name|{format!("{acc} {name},")})+"\nCommand format: /stats Name"
                        }
                        ))
                    },
                    Err(_)=>None
                };
                if let Some(message)=message{
                    let _=message.await;
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: serenity::all::Ready) {
        println!("{} is connected!", ready.user.name);
    }
}
    
#[derive(Deserialize,Debug)]
struct Config{
    excluded_modes:Vec<Mode>,
    excluded_lobbies:Vec<String>,
    results_dir:String,
    discord_token:String,
    updates_channel_ids:Vec<ChannelId>,
    recive_messages:bool,
    tracked_players:Vec<String>,
    s3s_path:Option<String>,
    nxapi_path:Option<String>,
    s3s_config_path:Option<String>,
    update_game_interval:Option<u64>,
}

fn start_auto_update(s3s_path:Option<String>,nxapi_path:Option<String>,s3s_config_path:Option<String>,update_minutes:u64,s3s_run_path:PathBuf){
    let start_time=Instant::now();
    // spawn game updater
    if let Some(s3s_path) =s3s_path 
    && !s3s_path.is_empty(){
        tokio::spawn(async move {
            let mut update_games_interval=tokio::time::interval(Duration::from_mins(update_minutes));
            update_games_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            update_games_interval.reset_after(
                if env::args().collect::<Vec<String>>().contains(&String::from("-s")){
                    update_games_interval.period()
                }else{
                    Duration::from_secs(30) // allow time for token to be updated
                });
            let s3s_path=PathBuf::from(s3s_path);
            loop{
                //update games
                let time=update_games_interval.tick().await;
                println!("updating games at {} mins since start ...",(time-start_time).as_secs()/60);
                let mut command=std::process::Command::new("python3");
                command.args(vec!(&s3s_path.to_string_lossy(),"-o"));
                command.current_dir(&s3s_run_path);
                let _=command.spawn();
            }
        });
    }
    
    if let Some(nxapi_path)=nxapi_path && !nxapi_path.is_empty()
    && let Some(s3s_config_path)=s3s_config_path && !s3s_config_path.is_empty(){
        //update tokens
        tokio::spawn(async move{
            let mut update_games_interval=tokio::time::interval(Duration::from_hours(20));
            update_games_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop{
                let time=update_games_interval.tick().await;
                println!("updating token at {} mins since start ...",(time-start_time).as_secs()/60);
                let _ =std::process::Command::new(&nxapi_path)
                .args(vec!("util","update-s3s-token",&s3s_config_path))
                .spawn();
            }

        });
    }
}