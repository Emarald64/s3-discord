use serde::{Deserialize, Serialize};
use std::{fmt::Display};
use std::str::FromStr;
use serde_json::{Value,Map};
use anyhow::{anyhow,bail};
use serenity::{all::{CreateEmbed, CreateMessage, CreateSelectMenu, CreateSelectMenuOption}, model::{Color, Timestamp}};

#[derive(Deserialize,Serialize,PartialEq, Eq,Debug, Hash, Clone, Copy)]
pub enum Mode{
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
        f.write_str(match self{
            Self::TurfWar=>"Turf War",
            Self::SplatZones=>"Splat Zones",
            Self::RainMaker=>"Rainmaker",
            Self::TowerControl=>"Tower Conrol",
            Self::ClamBlitz=>"Clam Blitz",
        })
    }
}

pub struct Gear{
    pub name:String,
    pub primary_ability:String,
    pub secondary_abilities:Vec<Option<String>>,
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
    pub fn display_secondary_ability(&self,idx:usize)->&str{
        match self.secondary_abilities.get(idx){
            Some(Some(name))=>name,
            Some(None)=>"???",
            None=>""
        }
    }
}

pub struct Player{
    // me:bool,
    pub name:String,
    pub name_id:String,
    pub byname:String,
    pub turf_inked:u16,
    // rank:u8,
    pub weapon:String,
    pub kills:u8,
    pub assists:u8,
    pub deaths:u8,
    pub specials:u8,
    pub gears:[Gear;3],
    pub battle_result:Option<BattleResult>,
}

impl Player{
    fn from_map(map:&Map<String,Value>, battle_result:Option<BattleResult>)->anyhow::Result<Self>{
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
                _=>0,
            },
            assists:match result.get("assist"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("assists not an int"))? as u8,
                _=>0,
            },
            deaths:match result.get("death"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("deaths not an int"))? as u8,
                _=>0,
            },
            specials:match result.get("special"){
                Some(Value::Number(n))=>n.as_u64().ok_or(anyhow!("specials not an int"))? as u8,
                _=>0,
            },
            gears:[
                Gear::from_map(map.get("headGear").ok_or(anyhow!("failed to find headgear"))?).ok_or(anyhow!("failed to build headgear"))?,
                Gear::from_map(map.get("clothingGear").ok_or(anyhow!("failed to find clothing"))?).ok_or(anyhow!("failed to build clothing"))?,
                Gear::from_map(map.get("shoesGear").ok_or(anyhow!("failed to find shoes"))?).ok_or(anyhow!("failed to build shoes"))?,
                ],

            battle_result:battle_result,
        })
    }
}

impl Display for Player{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{0:10} K:{1:2} A:{2:2} D:{3:2} S:{4:2} {5:4}p K/D:{6:.2}",self.name,self.kills,self.assists,self.deaths,self.specials,self.turf_inked,(self.kills as f32)/(self.deaths as f32))
    }
}

pub struct Stage{
    pub name:String,
    pub image_url:String,
}

impl Display for Stage{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BattleResult{
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


pub struct Battle{
    pub lobby:String,
    pub mode:Mode,
    pub stage:Stage,
    pub result:BattleResult,
    pub our_score:u8,
    pub their_score:u8,
    pub our_players:Vec<Player>,
    pub their_players:Vec<Player>,
    pub duration:u16,
    pub our_color:Color,
    // start_time:u64,
    // end_time:u64,
    pub timestamp:Timestamp,
}
impl Battle{
    pub fn from_map(map:Map<String,Value>)->anyhow::Result<Self>{
        let map=map.get("data").ok_or(anyhow!("failed to get data"))?.get("vsHistoryDetail").ok_or(anyhow!("failed to get vsHistory"))?;
        let mode=match map.get("vsRule"){
            Some(Value::Object(mode))=>match mode.get("name"){
                Some(Value::String(code))=>code.parse()?,
                _=>bail!("failed to find mode"),
            },
            _=>bail!("Failed to find mode"),
        };
        let result: BattleResult=match map.get("judgement"){
            Some(Value::String(s))=>s.to_lowercase().parse()?,
            _=>bail!("Failed to find result"),
        };
        let our_team=map.get("myTeam").ok_or(anyhow!("Couldn't find my team"))?;
        let their_team=map.get("otherTeams").ok_or(anyhow!("Couldn't find other teams"))?.get(0).ok_or(anyhow!("couldn't find other team"))?;
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
            result:result,
            our_score:{
                let result=our_team.get("result").ok_or(anyhow!("Couldn't find our result"))?;
                match mode{
                    Mode::TurfWar=>(result.get("paintRatio").ok_or(anyhow!("Couldn't find our paint"))?.as_f64().ok_or(anyhow!("our paint is not a float???"))?*100.0) as u8,
                    _=>result.get("score").ok_or(anyhow!("counldn't get our score"))?.as_u64().ok_or(anyhow!("our paint is not a int"))? as u8,
                } 
            },
            their_score: {
                let result=their_team.get("result").ok_or(anyhow!("Couldn't find our result"))?;
                match mode{
                    Mode::TurfWar=>(result.get("paintRatio").ok_or(anyhow!("couldn't find their paint"))?.as_f64().ok_or(anyhow!("their paint is not a float???"))?*100.0) as u8,
                    _=>result.get("score").ok_or(anyhow!("counldn't get their score"))?.as_u64().ok_or(anyhow!("their paint is not a int"))? as u8,
                } 
            },
            mode: mode,
            our_players: our_team.get("players").ok_or(anyhow!("couldn't get our players"))?.as_array().ok_or(anyhow!("our players not an array"))?.iter().filter_map(|player|{
                Player::from_map(player.as_object()?,Some(result)).ok()
            }).collect(), 
            their_players:their_team.get("players").ok_or(anyhow!("couldn't get their players"))?.as_array().ok_or(anyhow!("their players not an array"))?.iter().filter_map(|player|{
                Player::from_map(player.as_object()?,None).ok()
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

    pub fn to_message(&self,file_name:Option<&str>)->CreateMessage{
        let our_players=self.our_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let their_players=self.their_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let percent_if_turf_war=match self.mode{
            Mode::TurfWar=>"%",
            _=>"",
        };
        let out=CreateMessage::default().add_embed(
            CreateEmbed::default()
            .timestamp(&self.timestamp)
            .image(&self.stage.image_url)
            .title(format!("{2}: {0} - {1}",self.mode,&self.stage.name,&self.result))
            .description(format!("{4}:  {0}{percent_if_turf_war}-{1}{percent_if_turf_war}\nDuration {5}\nLobby: {6}\n```Our Players:\n{2}\nTheir Players:\n{3}```",self.our_score,self.their_score,our_players,their_players,self.result,self.format_durr(),self.lobby.to_lowercase()))
            .color(self.our_color)
        );
        match file_name{
            Some(file_name)=>{out.select_menu(CreateSelectMenu::new(file_name,serenity::all::CreateSelectMenuKind::String {options:self.our_players.iter().chain(self.their_players.iter()).map(|player|{CreateSelectMenuOption::new(&player.name,player.name.clone()+&player.name_id)}).collect()}).placeholder("Select a player for more info"))},
            None=>out
        }
    }

    fn format_durr(&self)->String{
        format!("{}:{:02}",self.duration/60,self.duration%60)
    }
}

impl Display for Battle{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let our_players=self.our_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let their_players=self.their_players.iter().fold(String::from(""),|acc,player|{format!("{0}{1}\n",acc,player)});
        let percent_if_turf_war=match self.mode{
            Mode::TurfWar=>"%",
            _=>"",
        };
        write!(f,"{0} : {1}\n{6}:  {2}{percent_if_turf_war}-{3}{percent_if_turf_war}\nDuration {7}\n\nOur Players:\n{4}\nTheir Players:\n{5}",self.mode,self.stage,self.our_score,self.their_score,our_players,their_players,self.result,self.format_durr())
    }
}