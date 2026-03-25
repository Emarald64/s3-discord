use tokio;
use reqwest::{Client,header};
use serde_json::{Map, Value};
use std::fmt::Display;
use std::str::FromStr;
use anyhow::{anyhow,bail};

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
                                    let battle=Battle::from_map(map)?;
                                    println!("{}",battle);
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

enum Stage{
    BAndD,
    Depot,
    Springs,
    Capital,
    Alley,
    Heights,
    GrandArena,
    Market,
    Bridge,
    Track,
    Academy,
    Hub,
    Resort,
    Mart,
    Manta,
    Airport,
    Metalworks,
    Museum,
    RomEn,
    Gorge,
    Cargo,
    Shipyard,
    Ruins,
    Spillway,
    Underpass,
    World
}
impl FromStr for Stage{
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Stage::*;
        match s{
            "taraport"=>Ok(BAndD),
            "negitoro"=>Ok(Depot),
            "kusaya"=>Ok(Springs),
            "takaashi"=>Ok(Capital),
            "gonzui"=>Ok(Alley),
            "hirame"=>Ok(Heights),
            "grand_arena"=>Ok(GrandArena),
            "yagara"=>Ok(Market),
            "masaba"=>Ok(Bridge),
            "kombu"=>Ok(Track),
            "amabi"=>Ok(Academy),
            "ryugu"=>Ok(Hub),
            "mahimahi"=>Ok(Resort),
            "zatou"=>Ok(Mart),
            "manta"=>Ok(Manta),
            "kajiki"=>Ok(Airport),
            "namero"=>Ok(Metalworks),
            "kinmedai"=>Ok(Museum),
            "baigai"=>Ok(RomEn),
            "yunohana"=>Ok(Gorge),
            "ohyo"=>Ok(Cargo),
            "chozame"=>Ok(Shipyard),
            "nampla"=>Ok(Ruins),
            "mategai"=>Ok(Spillway),
            "decaline"=>Ok(Underpass),
            "sumeshi"=>Ok(World),
            _=>bail!("Invalid Stage"),
        }
    }
}

impl Display for Stage{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Stage::*;
        f.write_str(
            match self{
                BAndD=>"Barnacle & Dime",
                Depot=>"Bluefin Depot",
                Springs=>"Brinewater Springs",
                Capital=>"Crableg Capital",
                Alley=>"Eeltail Alley",
                Heights=>"Flounder Heights",
                GrandArena=>"Grand Splatlands Bowl",
                Market=>"Hagglefish Market",
                Bridge=>"Hammerhead Bridge",
                Track=>"Humpback Pump Track",
                Academy=>"Inkblot Art Academy",
                Hub=>"Lemuria Hub",
                Resort=>"Mahi-Mahi Resort",
                Mart=>"MakoMart",
                Manta=>"Manta Maria",
                Airport=>"Marlin Airport",
                Metalworks=>"Mincemeat Metalworks",
                Museum=>"Museum d'Alfonsino",
                RomEn=>"Robo ROM-en",
                Gorge=>"Scorch Gorge",
                Cargo=>"Shipshape Cargo Co.",
                Shipyard=>"Sturgeon Shipyard",
                Ruins=>"Um'ami Ruins",
                Spillway=>"Undertow Spillway",
                Underpass=>"Urchin Underpass",
                World=>"Wahoo World",
            }
        )
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
    stage:Stage,
    result:BattleResult,
    our_score:u8,
    their_score:u8,
    our_players:Vec<Player>,
    their_players:Vec<Player>,
    // start_time:u64,
    // end_time:u64,
}
impl Battle{
    fn from_map(map:Map<String,Value>)->anyhow::Result<Self>{
        let mode=match map.get("mode"){
            Some(Value::String(string))=>Mode::from_str(string)?,
            _=>bail!("Failed to find mode"),
        };
        Ok(Battle { 
            uuid: match map.get("uuid"){
                Some(Value::String(string))=>string.clone(),
                _=>bail!("failed to find uuid"),
            },
            stage:match map.get("stage"){
                Some(Value::String(string))=>string.parse()?,
                _=>bail!("Failed to find stage"),
            }, 
            lobby:match map.get("lobby"){
                Some(Value::String(string))=>string.clone(),
                _=>bail!("Failed to find lobby"),
            },
            result:match map.get("result"){
                Some(Value::String(s))=>s.parse()?,
                _=>bail!("Failed to find result"),
            },
            our_score: 
                match map.get(match mode {
                    Mode::TurfWar=>"our_team_percent",
                        _=>"our_team_count",
                    })
                    {
                        Some(Value::Number(number))=>match number.as_u64(){
                            Some(n)=>n as u8,
                            None=>bail!("their score too high"),
                        },
                        _=>bail!("Failed to find our score"),
                    }, 
            their_score: 
                match map.get(match mode {
                    Mode::TurfWar=>"their_team_percent",
                    _=>"their_team_count",
                })
                {
                Some(Value::Number(number))=>match number.as_u64(){
                    Some(n)=>n as u8,
                    None=>bail!("their score too high"),
                },
                _=>bail!("Failed to find their score"),
            }, 
            mode: mode,
            our_players: match map.get("our_team_players"){
                    Some(Value::Array(players))=>players.iter().filter_map(|player|{
                        match player{
                            Value::Object(player)=>Player::from_map(player),
                            _=>return None,
                        }
                    }).collect::<Vec<_>>(),
                _=>bail!("Failed to find our players"),
            }, 
            their_players: match map.get("their_team_players"){
                Some(Value::Array(players))=>players.iter().filter_map(|player|{
                        match player{
                            Value::Object(player)=>Player::from_map(player),
                            _=>return None,
                        }
                }).collect::<Vec<_>>(),
                _=>bail!("Failed to find their players"),
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
        let our_players=self.our_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let their_players=self.their_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        write!(f,"{0} : {1}\n{6}   {2}-{3}\nOur Players\n{4}Their Players{5}",self.mode,self.stage,self.our_score,self.their_score,our_players,their_players,self.result)
    }
}