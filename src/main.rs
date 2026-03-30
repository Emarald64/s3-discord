use serenity::all::{CreateSelectMenu, CreateSelectMenuOption, Interaction};
use tokio;
use notify::{self, Event, EventKind, Watcher, event};
// use reqwest::{Client,header};
use serde_json::{Map, Value};
use std::time::Duration;
use std::{fmt::Display};
use std::str::FromStr;
use anyhow::{anyhow,bail};
use std::{sync::{mpsc,Arc},path::Path,fs::File};
use std::{env, fs};

use serenity::prelude::*;
use serenity::all::*;
use serenity::{builder::{CreateEmbed, CreateMessage}, model::{Timestamp,id::ChannelId,colour::Color,application::ComponentInteractionDataKind}};

use serenity::async_trait;

const S3S_RESULTS_DIR:&str="/home/agiller/.config/s3s/exports/results/";
const SEND_CHANNEL_ID:ChannelId=ChannelId::new(1481734356832882852);

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    //setup discord
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents=GatewayIntents::GUILD_MESSAGES;
    let mut client=Client::builder(&token, intents).event_handler(Handeler).await?;
    let http= Arc::clone(&client.http);
    tokio::task::spawn(async move {let _=client.start().await;});
    // client.start().await?;
    //setup notify to check s3s results folder
    let (tx,rx)=mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new(S3S_RESULTS_DIR), notify::RecursiveMode::NonRecursive)?;
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
                // if the file is more than a second old, dont wait
                if let Some(wait)=wait{
                    tokio::time::sleep(wait).await;
                }
                println!("scanning file at {}",path.to_string_lossy());
                // let path="/home/agiller/.config/s3s/exports/results/20260310T205332Z.json";
                let file=File::open(path).expect("invalid file path");
                if let Value::Object(battle)=serde_json::from_reader(file)?{
                    println!("parsing battle");
                    match Battle::from_map(battle){
                        Ok(battle)=>{
                            println!("{}",battle);
        
                            // post log to discord
                            SEND_CHANNEL_ID.send_message(&http, message_from_battle(&battle,String::from(path.file_name().expect("File path ends in ..").to_str().expect("string is not valid utf-8")))).await?;
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


fn message_from_battle(battle:&Battle,file_name:String)->CreateMessage{
    let our_players=battle.our_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
    let their_players=battle.their_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
    let percent_if_turf_war=match battle.mode{
        Mode::TurfWar=>"%",
        _=>"",
    };
    CreateMessage::default().add_embed(
        CreateEmbed::default()
        .timestamp(&battle.timestamp)
        .image(&battle.stage.image_url)
        .title(format!("{2}: {0} - {1}",battle.mode,&battle.stage.name,&battle.result))
        .description(format!("{4}:  {0}{percent_if_turf_war}-{1}{percent_if_turf_war}\nDuration {5}\nLobby: {6}\n```Our Players:\n{2}\nTheir Players:\n{3}```",battle.our_score,battle.their_score,our_players,their_players,battle.result,format_durr(battle.duration),battle.lobby.to_lowercase()))
        .color(battle.our_color)
    )
    .select_menu(CreateSelectMenu::new(file_name,serenity::all::CreateSelectMenuKind::String {options:battle.our_players.iter().chain(battle.their_players.iter()).map(|player|{CreateSelectMenuOption::new(&player.name,player.name.clone()+&player.name_id)}).collect()}).placeholder("Select a player for more info"))
}
    struct Handeler;
    
    #[async_trait]
    impl EventHandler for Handeler{
        async fn interaction_create(&self,ctx:Context,interaction:Interaction){
            // dbg!(&interaction);
            if let Interaction::Component(mut interaction)=interaction{
                let data=&interaction.data;
                if let ComponentInteractionDataKind::StringSelect{values:data_values}=&data.kind{
                    println!("opening {}",&data.custom_id);
                    if let Ok(file)=File::open(String::from(S3S_RESULTS_DIR)+&data.custom_id){
                        if let Ok(Value::Object(battle))=serde_json::from_reader(file){
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
                    }
                }
            }
        }
        async fn ready(&self, _: Context, ready: serenity::all::Ready) {
            println!("{} is connected!", ready.user.name);
        }
    }
    
enum Mode{
    TurfWar,
    TowerControl,
    SplatZones,
    RainMaker,
    ClamBlitz,
}

impl FromStr for Mode{
    type Err=anyhow::Error;
    fn from_str(s:&str)->Result<Self,Self::Err>{
        match s{
            "Turf War"=>Ok(Mode::TurfWar),
            "Splat Zones"=>Ok(Mode::SplatZones),
            "Rainmaker"=>Ok(Mode::RainMaker),
            "Tower Conrol"=>Ok(Mode::TowerControl),
            "Clam Blitz"=>Ok(Mode::ClamBlitz),
            _=>Err(anyhow!("failed to parse mode")),
        }
    }
}

impl Display for Mode{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::TurfWar=>f.write_str("Turf War"),
            Self::SplatZones=>f.write_str("Splat Zones"),
            Self::RainMaker=>f.write_str("Rainmaker"),
            Self::TowerControl=>f.write_str("Tower Conrol"),
            Self::ClamBlitz=>f.write_str("Clam Blitz"),
        }
    }
}

struct Gear{
    name:String,
    primary_ability:String,
    secondary_abilities:Vec<Option<String>>,
}
impl Gear{
    fn from_map(map:&Value)->Option<Self>{
        Some(Gear{
            name:map.get("name")?.as_str()?.to_string(),
            primary_ability:String::from(map.get("primaryGearPower")?.get("name")?.as_str()?),
            secondary_abilities:map.get("additionalGearPowers")?.as_array()?.iter().map_while(|gear|{
                match gear.get("name"){
                    Some(Value::String(name))=>match name.as_str(){
                        "Unknown"=>Some(None),
                        name=>Some(Some(String::from(name)))
                    },
                    _=>None
                }
            }).collect()
        })
    }
    fn display_secondary_ability(&self,idx:usize)->&str{
        match self.secondary_abilities.get(idx){
            Some(Some(name))=>name,
            Some(None)=>"???",
            None=>""
        }
    }
}

struct Player{
    // me:bool,
    name:String,
    name_id:String,
    byname:String,
    turf_inked:u16,
    // rank:u8,
    weapon:String,
    kills:u8,
    assists:u8,
    deaths:u8,
    specials:u8,
    gears:[Gear;3],
}

impl Player{
    fn from_map(map:&Map<String,Value>)->anyhow::Result<Self>{
        let result=match map.get("result"){
            Some(Value::Object(map))=>map,
            _=>bail!("Failed to get player result"),
        };
        Ok(Player{
            // me:if let Some(Value::Bool(me)) = map.get("isMyself") {*me} else {false},
            name_id:match map.get("nameId").ok_or(anyhow!("Failed to get player id"))?{
                Value::String(id)=>id.clone(),
                _=>bail!("player id is not string")
            },
            name:match map.get("name"){
                Some(Value::String(n))=>n.clone(),
                _=>bail!("Failed to get player name"),
            },
            byname:String::from(map.get("byname").ok_or(anyhow!("Failed to find byname"))?.as_str().ok_or(anyhow!("byname is not string"))?),
            turf_inked:match map.get("paint"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("too much paint"))? as u16,
                _=>bail!("Failed to get player paint"),
            },
            // rank:match map.get("rank_in_team")?{
            //     Value::Number(n)=>n.as_u64()? as u8,
            //     _=>return None,
            // },
            weapon:match map.get("weapon"){
                Some(Value::Object(weapon))=>match weapon.get("name"){
                    Some(Value::String(name))=>name.clone(),
                    _=>bail!("Failed to get weapon name")
                },
                _=>bail!("Failed to get weapon")
            },
            kills:match result.get("kill"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("kills not an int"))? as u8,
                _=>bail!("Failed to get kills"),
            },
            assists:match result.get("assist"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("assists not an int"))? as u8,
                _=>bail!("Failed to get assits"),
            },
            deaths:match result.get("death"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("deaths not an int"))? as u8,
                _=>bail!("Failed to get deaths"),
            },
            specials:match result.get("special"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("specials not an int"))? as u8,
                _=>bail!("Failed to get specials"),
            },
            gears:[
                Gear::from_map(map.get("headGear").ok_or(anyhow!("failed to find headgear"))?).ok_or(anyhow!("failed to build headgear"))?,
                Gear::from_map(map.get("clothingGear").ok_or(anyhow!("failed to find clothing"))?).ok_or(anyhow!("failed to build clothing"))?,
                Gear::from_map(map.get("shoesGear").ok_or(anyhow!("failed to find shoes"))?).ok_or(anyhow!("failed to build shoes"))?,
                ],
        })
    }
}

impl Display for Player{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{0:10} K:{1:2} A:{2:2} D:{3:2} S:{4:2} {5:4}p",self.name,self.kills,self.assists,self.deaths,self.specials,self.turf_inked)
    }
}


struct Stage{
    name:String,
    image_url:String,
}

impl Display for Stage{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

enum BattleResult{
    Win,
    Lose,
    Draw,
    ExemptedLose,
    Unknown,
}

impl FromStr for BattleResult{
    type Err=anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s{
            "win"=>Ok(Self::Win),
            "lose"=>Ok(Self::Lose),
            "draw"=>Ok(Self::Draw),
            "exempted_lose"=>Ok(Self::ExemptedLose),
            _=>Ok(Self::Unknown)
        }
    }
}

impl Display for BattleResult{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::Win=>f.write_str("Win"),
            Self::Lose=>f.write_str("Lose"),
            Self::Draw=>f.write_str("Draw"),
            Self::ExemptedLose=>f.write_str("Exempted Loss"),
            Self::Unknown=>f.write_str("Unknown"),
        }
    }
}

struct Battle{
    lobby:String,
    mode:Mode,
    stage:Stage,
    result:BattleResult,
    our_score:u8,
    their_score:u8,
    our_players:Vec<Player>,
    their_players:Vec<Player>,
    duration:u16,
    our_color:Color,
    // start_time:u64,
    // end_time:u64,
    timestamp:Timestamp,
}
impl Battle{
    fn from_map(map:Map<String,Value>)->anyhow::Result<Self>{
        let map=map.get("data").ok_or(anyhow!("failed to get data"))?.get("vsHistoryDetail").ok_or(anyhow!("failed to get vsHistory"))?;
        let mode=match map.get("vsRule"){
            Some(Value::Object(mode))=>match mode.get("name"){
                Some(Value::String(code))=>code.parse()?,
                _=>bail!("failed to find mode"),
            },
            _=>bail!("Failed to find mode"),
        };
        let our_team=map.get("myTeam").ok_or(anyhow!("Couldn't find my team"))?;
        let their_team=map.get("otherTeams").ok_or(anyhow!("Couldn't find my team"))?.get(0).unwrap();
        Ok(Battle { 
            // file_name:file_name.into(),
            stage:{
                let vs_stage=map.get("vsStage").ok_or(anyhow!("Failed to find stage"))?;
                Stage{
                    name:String::from(vs_stage.get("name").ok_or(anyhow!("Failed to get stage name"))?.as_str().ok_or(anyhow!("Stage name is not string"))?),
                    image_url:String::from(vs_stage.get("image").ok_or(anyhow!("failed to find stage image"))?.get("url").ok_or(anyhow!("failed to find stage image url"))?.as_str().ok_or(anyhow!("stage image url not string"))?)
                }
            },
            lobby:String::from(map.get("vsMode").ok_or(anyhow!("couldn't find vsMode"))?.get("mode").ok_or(anyhow!("mode not found"))?.as_str().ok_or(anyhow!("lobby is not a string"))?),
            result:match map.get("judgement"){
                Some(Value::String(s))=>s.to_lowercase().parse()?,
                _=>bail!("Failed to find result"),
            },
            our_score:{
                let result=our_team.get("result").ok_or(anyhow!("Couldn't find our result"))?;
                match mode{
                    Mode::TurfWar=>result.get("paintRatio").map_or(0,|paint|{(paint.as_f64().unwrap()*100.0) as u8}),
                    _=>result.get("score").ok_or(anyhow!("counldn't get our score"))?.as_u64().unwrap() as u8,
                } 
            },
            their_score: {
                let result=their_team.get("result").ok_or(anyhow!("Couldn't find our result"))?;
                match mode{
                    Mode::TurfWar=>result.get("paintRatio").map_or(0,|paint|{(paint.as_f64().unwrap()*100.0) as u8}),
                    _=>result.get("score").ok_or(anyhow!("counldn't get our score"))?.as_u64().unwrap() as u8,
                } 
            },
            mode: mode,
            our_players: our_team.get("players").ok_or(anyhow!("couldn't get our players"))?.as_array().unwrap().iter().filter_map(|player|{
                Player::from_map(player.as_object()?).ok()
            }).collect(), 
            their_players:their_team.get("players").ok_or(anyhow!("couldn't get their players"))?.as_array().unwrap().iter().filter_map(|player|{
                Player::from_map(player.as_object()?).ok()
            }).collect(), 
            our_color:{
                let color=our_team.get("color").ok_or(anyhow!("Couldn't get our color"))?;
                Color::from_rgb(
                    (color.get("r").ok_or(anyhow!("couldn't find red value"))?.as_f64().ok_or(anyhow!("red is not a float"))? * 256.0) as u8, 
                    (color.get("g").ok_or(anyhow!("couldn't find green value"))?.as_f64().ok_or(anyhow!("green is not a float"))? * 256.0) as u8, 
                    (color.get("b").ok_or(anyhow!("couldn't find blue value"))?.as_f64().ok_or(anyhow!("blue is not a float"))? * 256.0) as u8,
                )
            },
            duration:map.get("duration").ok_or(anyhow!("Failed to get duration"))?.as_u64().ok_or(anyhow!("duration is not integer"))? as u16,
            timestamp:String::from(map.get("playedTime").ok_or(anyhow!("failed to get playedTime"))?.as_str().ok_or(anyhow!("playedTime not string"))?).parse()?,
        })
    }
}

fn format_durr(durr:u16)->String{
    format!("{}:{:02}",durr/60,durr%60)
}

impl Display for Battle{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let our_players=self.our_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let their_players=self.their_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let percent_if_turf_war=match self.mode{
            Mode::TurfWar=>"%",
            _=>"",
        };
        write!(f,"{0} : {1}\n{6}:  {2}{percent_if_turf_war}-{3}{percent_if_turf_war}\nDuration {7}\n\nOur Players:\n{4}\nTheir Players:\n{5}",self.mode,self.stage,self.our_score,self.their_score,our_players,their_players,self.result,format_durr(self.duration))
    }
}