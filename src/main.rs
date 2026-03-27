use tokio;
use notify::{self,Event, Watcher,EventKind,event};
// use reqwest::{Client,header};
use serde_json::{Map, Value};
use std::fmt::Display;
use std::str::FromStr;
use anyhow::{anyhow,bail};
use std::{sync::mpsc,path::Path,fs::File};

const S3S_RESULTS_DIR:&str="";

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    //setup notify to check s3s results folder
    // let (tx,rx)=mpsc::channel::<notify::Result<Event>>();
    // let mut watcher = notify::recommended_watcher(tx)?;
    // watcher.watch(Path::new(S3S_RESULTS_DIR), notify::RecursiveMode::NonRecursive)?;
    // loop{
    //     // wait for new log
    //     match rx.recv(){
    //         Ok(Ok(event))=>if let EventKind::Access(event::AccessKind::Close(_))=event.kind{
                // let path=event.paths[0].as_path();
                let path="/home/agiller/.config/s3s/exports/results/20260310T205332Z.json";
                let file=File::open(path)?;
                if let Value::Object(battle)=serde_json::from_reader(file)?{
                    let battle=Battle::from_map(battle)?;
                    println!("{}",battle);
                }
        //     },
        //     Ok(Err(e))=>println!("watch error: {:?}", e),
        //     Err(e)=>bail!(e)
        // };
        // check stats.ink battle log
        // match client.get("https://stat.ink/api/v3/s3s/uuid-list?lobby=private").bearer_auth(API_KEY).send().await{
        //     Ok(res)=>{
        //         if let Value::Array(battles)=res.json().await?{
        //             let battles:Vec<&String>=battles.iter().filter_map(|value|{
        //                 match value {
        //                     Value::String(uuid)=>Some(uuid),
        //                     _=>None,
        //                 }
        //             }).collect();
        //             let mut i=0;
        //             while i<battles.len() && {
        //                 match most_recent_battle{
        //                     Some(uuid)=>battles[i]!=uuid,
        //                     _=>true
        //                 }   
        //             }{
        //                 // get battle log
        //                 match client.get(format!("https://stat.ink/api/v3/battle/{}",battles[i])).send().await{
        //                     Ok(res)=>{
        //                         if let Value::Object(map)=res.json().await?{
        //                             let battle=Battle::from_map(map)?;
        //                             println!("{}",battle);
        //                         }
        //                     }
        //                     Err(err)=>{
        //                         println!("error: {}",err);
        //                     }
        //                 }
        //             }
        //             most_recent_battle=Some(battles[0]);
        //         }
        //     }
        //     Err(err)=>{
        //         println!("error: {}",err);
        //     }
        
        // parse log
        
        // post log to discord
        Ok(())
    // }
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
    // rank:u8,
    weapon:String,
    kills:u8,
    assists:u8,
    deaths:u8,
    specials:u8,
    // gears:Vec<Gear>,
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
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("too many kills"))? as u8,
                _=>bail!("Failed to get kills"),
            },
            assists:match result.get("assist"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("too many assists"))? as u8,
                _=>bail!("Failed to get assits"),
            },
            deaths:match result.get("death"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("too many deaths"))? as u8,
                _=>bail!("Failed to get deaths"),
            },
            specials:match result.get("special"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("too many specials"))? as u8,
                _=>bail!("Failed to get specials"),
            },
            // gears:match map.get("gears"){
            //     Some(Value::Object(gears))=>{
            //         gears.values().map_while(|gear|{
            //             if let Value::Object(gear)=gear{
            //                 Some(Gear{
            //                     primary_ability:
            //                         match gear.get("primary_ability"){
            //                             Some(Value::String(ability))=>ability.clone(),
            //                             _=>{
            //                                 return None
            //                             }
            //                         },
            //                     secondary_abilities:
            //                         match gear.get("secondary_abilities"){
            //                             Some(Value::Array(abilities))=>abilities.iter().map_while(|ability|{
            //                                 match ability{
            //                                     Value::String(ability)=>Some(Some(ability.clone())),
            //                                     Value::Null=>Some(None),
            //                                     _=>None,
            //                                 }
            //                             }).collect(),
            //                             _=>{
            //                                 return None
            //                             }
            //                         }
            //                 })
            //             }else{
            //                 return None
            //             }
            //         }).collect()
            //     },
            //     _=>bail!("failed to find gears"),
            // },
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
            "Barnacle & Dime"=>Ok(BAndD),
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
    // uuid:String,
    // lobby:String,
    mode:Mode,
    stage:String,
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
        let map=map.get("data").ok_or(anyhow!("failed to get data"))?.get("vsHistoryDetail").ok_or(anyhow!("failed to get vsHistory"))?;
        let mode=match map.get("vsRule"){
            Some(Value::Object(mode))=>match mode.get("rule"){
                Some(Value::String(code))=>code.to_lowercase().parse()?,
                _=>bail!("failed to find mode"),
            },
            _=>bail!("Failed to find mode"),
        };
        Ok(Battle { 
            // uuid: match map.get("uuid"){
            //     Some(Value::String(string))=>string.clone(),
            //     _=>bail!("failed to find uuid"),
            // },
            stage:String::from(map.get("vsStage").ok_or(anyhow!("Failed to find stage"))?.get("name").ok_or(anyhow!("Failed to get stage name"))?.as_str().ok_or(anyhow!("Stage name is not string"))?),
            // lobby:match map.get("lobby"){
            //     Some(Value::String(string))=>string.clone(),
            //     _=>bail!("Failed to find lobby"),
            // },
            result:match map.get("judgement"){
                Some(Value::String(s))=>s.to_lowercase().parse()?,
                _=>bail!("Failed to find result"),
            },
            our_score: map.get("myTeam").ok_or(anyhow!("Couldn't find my team"))?.get("result").ok_or(anyhow!("Couldn't find our result"))?.get("score").ok_or(anyhow!("counldn't get our score"))?.as_u64().unwrap() as u8,
            their_score: map.get("otherTeams").ok_or(anyhow!("Couldn't find my team"))?.get(0).unwrap().get("result").ok_or(anyhow!("Couldn't find our result"))?.get("score").ok_or(anyhow!("counldn't get our score"))?.as_u64().unwrap() as u8,
            mode: mode,
            our_players: map.get("myTeam").ok_or(anyhow!("couldn't find our team"))?.get("players").ok_or(anyhow!("couldn't get our players"))?.as_array().unwrap().iter().filter_map(|player|{
                Player::from_map(player.as_object()?).ok()
            }).collect(), 
            their_players:map.get("otherTeams").ok_or(anyhow!("Couldn't find my team"))?.get(0).unwrap().get("players").ok_or(anyhow!("couldn't get their players"))?.as_array().unwrap().iter().filter_map(|player|{
                Player::from_map(player.as_object()?).ok()
            }).collect(), 
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
        write!(f,"{0} : {1}\n{6}:  {2}-{3}\n\nOur Players:\n{4}\nTheir Players:\n{5}",self.mode,self.stage,self.our_score,self.their_score,our_players,their_players,self.result)
    }
}