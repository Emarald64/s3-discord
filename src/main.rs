use tokio;
use notify::{self, Event, EventKind, Watcher, event};
// use reqwest::{Client,header};
use serde_json::{Map, Value};
use std::{fmt::Display};
use std::str::FromStr;
use anyhow::{anyhow,bail};
use std::{sync::mpsc,path::Path,fs::File};
use std::env;

use serenity::{builder::{CreateEmbed, CreateMessage}, model::{Timestamp,id::ChannelId}};
use serenity::prelude::*;

const S3S_RESULTS_DIR:&str="/home/agiller/.config/s3s/exports/results/";
const SEND_CHANNEL_ID:ChannelId=ChannelId::new(1481734356832882852);

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    //setup discord
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents=GatewayIntents::GUILD_MESSAGES;
    let mut client=Client::builder(&token, intents).await?;
    //setup notify to check s3s results folder
    let (tx,rx)=mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new(S3S_RESULTS_DIR), notify::RecursiveMode::NonRecursive)?;
    loop{
        // wait for new log
        match rx.recv(){
            Ok(Ok(event))=>if let EventKind::Create(event::CreateKind::File)=event.kind{
                let path=event.paths[0].as_path();
                println!("scanning file at {}",path.to_string_lossy());
                // let path="/home/agiller/.config/s3s/exports/results/20260310T205332Z.json";
                let file=File::open(path).expect("invalid file path");
                if let Value::Object(battle)=serde_json::from_reader(file)?{
                    println!("parsing battle");
                    let battle=Battle::from_map(battle)?;
                    println!("parsed battle");
                    println!("{}",battle);

                    // post log to discord
                    SEND_CHANNEL_ID.send_message(&client.http, message_from_battle(&battle)).await?;
                }
            },
            Ok(Err(e))=>println!("watch error: {:?}", e),
            Err(e)=>bail!(e)
        };
    };
}


fn message_from_battle(battle:&Battle)->CreateMessage{
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
        .description(format!("{4}:  {0}{percent_if_turf_war}-{1}{percent_if_turf_war}\nDuration {5}\n\nOur Players:\n{2}\nTheir Players:\n{3}",battle.our_score,battle.their_score,our_players,their_players,battle.result,format_durr(battle.duration)))
    )
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
            primary_ability:map.get("primaryGearPower")?.get("name")?.to_string(),
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
}

struct Player{
    me:bool,
    name:String,
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
            me:if let Some(Value::Bool(me)) = map.get("isMyself") {*me} else {false},
            name:match map.get("name"){
                Some(Value::String(n))=>n.clone(),
                _=>bail!("Failed to get player name"),
            },
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
    // uuid:String,
    // lobby:String,
    mode:Mode,
    stage:Stage,
    result:BattleResult,
    our_score:u8,
    their_score:u8,
    our_players:Vec<Player>,
    their_players:Vec<Player>,
    duration:u16,
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
            // uuid: match map.get("uuid"){
            //     Some(Value::String(string))=>string.clone(),
            //     _=>bail!("failed to find uuid"),
            // },
            stage:{
                let vs_stage=map.get("vsStage").ok_or(anyhow!("Failed to find stage"))?;
                Stage{
                    name:String::from(vs_stage.get("name").ok_or(anyhow!("Failed to get stage name"))?.as_str().ok_or(anyhow!("Stage name is not string"))?),
                    image_url:String::from(vs_stage.get("image").ok_or(anyhow!("failed to find stage image"))?.get("url").ok_or(anyhow!("failed to find stage image url"))?.as_str().ok_or(anyhow!("stage image url not string"))?)
                }
            },
            // lobby:match map.get("lobby"){
            //     Some(Value::String(string))=>string.clone(),
            //     _=>bail!("Failed to find lobby"),
            // },
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
            duration:map.get("duration").ok_or(anyhow!("Failed to get duration"))?.as_u64().ok_or(anyhow!("duration is not integer"))? as u16,
            // start_time: match map.get("start_time")?{
            //     Value::Number(n)=>n.as_u64()?,
            //     _=>return None,
            // }, 
            // end_time: match map.get("end_time")?{
            //     Value::Number(n)=>n.as_u64()?,
            //     _=>return None,
            // }, 
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