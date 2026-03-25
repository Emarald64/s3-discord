use tokio;
use reqwest::{Client,header};
use serde_json::{Map, Value};
use std::fmt::Display;
use std::str::FromStr;
use anyhow::anyhow;

const API_KEY:&str="";

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    //setup reqwest for stats.ink
    let mut headers=header::HeaderMap::new();
    let mut auth_value=header::HeaderValue::from_str(API_KEY)?;
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);
    let client=Client::builder()
        // .default_headers(headers)
        .build()?;
    let mut most_recent_battle:Option<&str>=None;
    // loop{
        // wait
        // check stats.ink battle log
        match client.get("https://stat.ink/api/v3/s3s/uuid-list?lobby=private").bearer_auth(API_KEY).send().await{
            Ok(res)=>{
                if let Value::Array(battles)=res.json().await?{
                    let battles:Vec<&String>=battles.iter().filter_map(|value|{
                        match value {
                            Value::String(uuid)=>Some(uuid),
                            _=>None,
                        }
                    }).collect();
                    let mut i=0;
                    while i<battles.len() && {
                        match most_recent_battle{
                            Some(uuid)=>battles[i]!=uuid,
                            _=>true
                        }   
                    }{
                        // get battle log
                        match client.get(format!("https://stat.ink/api/v3/battle/{}",battles[i])).send().await{
                            Ok(res)=>{
                                if let Value::Object(map)=res.json().await?{
                                    let battle=Battle::from_map(map).ok_or(anyhow::anyhow!("Json parse error"))?;
                                }
                            }
                            Err(err)=>{
                                println!("error: {}",err);
                            }
                        }
                    }
                    most_recent_battle=Some(battles[0]);
                }
            }
            Err(err)=>{
                println!("error: {}",err);
            }
        };
        Ok(())
    
        // parse log
    
        // post log to discord
    // }
}


enum Mode{
    TurfWar,
    TowerControl,
    SplatZones,
    RainMaker,
    ClamBlitz,
}

struct ParseModeError;

impl FromStr for Mode{
    type Err=anyhow::Error;
    fn from_str(s:&str)->Result<Self,Self::Err>{
        match s{
            "nawabari"=>Ok(Mode::TurfWar),
            "area"=>Ok(Mode::SplatZones),
            "hoko"=>Ok(Mode::RainMaker),
            "yagura"=>Ok(Mode::TowerControl),
            "asari"=>Ok(Mode::ClamBlitz),
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
    primary_ability:String,
    secondary_abilities:Vec<Option<String>>,
}

// struct Gears{
//     headgear:Gear,
//     clothing:Gear,
//     shoes:Gear,
// }

struct Player{
    me:bool,
    name:String,
    turf_inked:u16,
    rank:u8,
    kills:u8,
    assists:u8,
    deaths:u8,
    specials:u8,
    gears:Vec<Gear>,
}

impl Player{
    fn from_map(map:&Map<String,Value>)->Option<Self>{
        Some(Player{
            me:if let Some(Value::String(s)) = map.get("me") {s=="yes"} else {false},
            name:match map.get("name")?{
                Value::String(n)=>n.clone(),
                _=>return None,
            },
            turf_inked:match map.get("inked")?{
                Value::Number(n)=>n.as_u64()? as u16,
                _=>return None,
            },
            rank:match map.get("rank_in_team")?{
                Value::Number(n)=>n.as_u64()? as u8,
                _=>return None,
            },
            kills:match map.get("kill")?{
                Value::Number(n)=>n.as_u64()? as u8,
                _=>return None,
            },
            assists:match map.get("assist")?{
                Value::Number(n)=>n.as_u64()? as u8,
                _=>return None,
            },
            deaths:match map.get("death")?{
                Value::Number(n)=>n.as_u64()? as u8,
                _=>return None,
            },
            specials:match map.get("special")?{
                Value::Number(n)=>n.as_u64()? as u8,
                _=>return None,
            },
            gears:match map.get("gears"){
                Some(Value::Object(gears))=>{
                    gears.values().map_while(|gear|{
                        if let Value::Object(gear)=gear{
                            Some(Gear{
                                primary_ability:
                                    match gear.get("primary_ability"){
                                        Some(Value::String(ability))=>ability.clone(),
                                        _=>{
                                            return None
                                        }
                                    },
                                secondary_abilities:
                                    match gear.get("secondary_abilities"){
                                        Some(Value::Array(abilities))=>abilities.iter().map_while(|ability|{
                                            match ability{
                                                Value::String(ability)=>Some(Some(ability.clone())),
                                                Value::Null=>Some(None),
                                                _=>None,
                                            }
                                        }).collect(),
                                        _=>{
                                            return None
                                        }
                                    }
                            })
                        }else{
                            return None
                        }
                    }).collect()
                },
                _=>return None,
            },
        })
    }
}

impl Display for Player{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{0} K:{1} A:{2} D:{3} S:{4} {5}p",self.name,self.kills,self.assists,self.deaths,self.specials,self.turf_inked)
    }
}

enum BattleResult{
    Win,
    Lose,
    Draw,
    ExemptedLose,
}

impl FromStr for BattleResult{
    type Err=anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s{
            "win"=>Ok(Self::Win),
            "lose"=>Ok(Self::Lose),
            "draw"=>Ok(Self::Draw),
            "exempted_lose"=>Ok(Self::ExemptedLose),
            _=>Err(anyhow!("Invalid game result"))
        }
    }
}

impl Display for BattleResult{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::Win=>f.write_str("Win"),
            Self::Lose=>f.write_str("Lose"),
            Self::Draw=>f.write_str("Draw"),
            Self::ExemptedLose=>f.write_str("Exempted Loss")
        }
    }
}

struct Battle{
    uuid:String,
    lobby:String,
    mode:Mode,
    stage:String,
    our_score:u8,
    their_score:u8,
    our_players:Vec<Player>,
    their_players:Vec<Player>,
    // start_time:u64,
    // end_time:u64,
}
impl Battle{
    fn from_map(map:Map<String,Value>)->anyhow::Result<Self>{
        let mode=match map.get("mode")?{
            Value::String(string)=>Mode::from_str(string)?,
            _=>return None,
        };
        Ok(Battle { 
            uuid: match map.get("uuid")?{
                Value::String(string)=>string.clone(),
                _=>return None,
            },
            stage:match map.get("stage")?{
                Value::String(string)=>string.clone(),
                _=>return None,
            }, 
            lobby:match map.get("lobby")?{
                Value::String(string)=>string.clone(),
                _=>return None,
            },
            our_score: 
                match map.get(match mode {
                    Mode::TurfWar=>"our_team_percent",
                        _=>"our_team_count",
                    })?
                    {
                        Value::Number(number)=>number.as_u64()? as u8,
                        _=>return None,
                    } 
                    , 
                    their_score: 
                    match map.get(match mode {
                        Mode::TurfWar=>"their_team_percent",
                        _=>"their_team_count",
                    })?
                    {
                    Value::Number(number)=>number.as_u64()? as u8,
                    _=>return None,
                } , 
            mode: mode,
            our_players: match map.get("our_team_players")?{
                    Value::Array(players)=>players.iter().filter_map(|player|{
                        match player{
                            Value::Object(player)=>Player::from_map(player),
                            _=>return None,
                        }
                    }).collect::<Vec<_>>(),
                _=>return None,
            }, 
            their_players: match map.get("their_team_players")?{
                    Value::Array(players)=>players.iter().filter_map(|player|{
                        match player{
                            Value::Object(player)=>Player::from_map(player),
                            _=>return None,
                        }
                    }).collect::<Vec<_>>(),
                _=>return None,
            },  
            // start_time: match map.get("start_time")?{
            //     Value::Number(n)=>n.as_u64()?,
            //     _=>return None,
            // }, 
            // end_time: match map.get("end_time")?{
            //     Value::Number(n)=>n.as_u64()?,
            //     _=>return None,
            // }, 
        })
    }
}

impl Display for Battle{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{0} : {1}",self.mode,self.stage)
    }
}