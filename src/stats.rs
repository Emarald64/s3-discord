use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serenity::all::Timestamp;
use tokio::task::JoinSet;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::AddAssign;
use crate::battle::*;
// use std::path::Path;
use tokio::{fs as tfs, io::AsyncReadExt};
use std::sync::{Arc, Mutex};
use std::fs::{File,self};
use anyhow;
use serde_json;

const STATS_PATH:&str="stats.json";

// #[derive(Serialize,Deserialize,Default,Debug)]
// pub struct AllStats{
//     player_stats:HashMap<String,TotalPlayerStats>,
//     wins:u32,
//     losses:u32,
//     time_spent:u32,
// }

pub async fn from_past_games(results_dir: &str, tracked_players:Vec<String>)->anyhow::Result<HashMap<String,TotalPlayerStats>>{
        let stats=Arc::new(Mutex::new(HashMap::new()));
        let mut tasks=JoinSet::new();
        let tracked_players=Arc::new(tracked_players);
        for entry in fs::read_dir(results_dir)?{
            if let Ok(entry)=entry{
                let tracked_players=Arc::clone(&tracked_players);
                let stats=Arc::clone(&stats);
                tasks.spawn(async move{
                    if let Ok(mut file)=tfs::File::open(entry.path()).await{
                        let mut buf=Vec::new();
                        if let Ok(_)=file.read_to_end(&mut buf).await
                        && let Ok(map)=serde_json::from_slice(buf.as_slice())
                        && let Ok(battle)=Battle::from_map(map){
                            let mut stats=stats.lock().unwrap();
                            add_game(&mut stats,&battle, &tracked_players);
                        }
                    }
                });
            }
        }
        tasks.join_all().await;
        let stats=Arc::into_inner(stats).expect("not all stats instances were dropped").into_inner()?;
        let _ = write_tracked_stats(&stats);
        Ok(stats)
    }

pub fn add_game(stats:&mut HashMap<String,TotalPlayerStats>,battle:&Battle,tracked_players:&Vec<String>){
    for player in &battle.our_players{
        if tracked_players.contains(&player.name){
            stats.entry(player.name.to_uppercase()).or_insert(TotalPlayerStats::default()).add_game(&player, battle.mode, battle.timestamp);
        }
    }
}



#[derive(Deserialize,Serialize,Default,Debug)]
pub struct TotalPlayerStats{
    total_stats:StatBreakdown,
    todays_stats:StatBreakdown,
    stats_date:NaiveDate,
    weapon_stats:HashMap<String,StatBreakdown>,
    mode_stats:HashMap<Mode,StatBreakdown>
}

impl TotalPlayerStats{
    fn add_game(&mut self,player:&Player,mode:Mode,battle_timestamp:Timestamp){
        self.total_stats+=player;
        self.weapon_stats.entry(player.weapon.clone()).and_modify(|weapon_stat|{*weapon_stat+=player;}).or_insert(StatBreakdown::from(player));
        self.mode_stats.entry(mode).and_modify(|mode_stat|{*mode_stat+=player;}).or_insert(StatBreakdown::from(player));
        self.check_for_old_day_stats();
        if (battle_timestamp.with_timezone(&chrono::Local).date_naive())==chrono::Local::now().date_naive(){
            self.todays_stats+=player;
        }
    }

    pub fn to_string(&mut self)->String{
        const HEADER:&str="Games   Win%  K/G   A/G   D/G   S/G   K/D";
        let mut weapon_stats:Vec<(&String,&StatBreakdown)>=self.weapon_stats.iter().collect();
        weapon_stats.sort_by_key(|stat|{u32::MAX-stat.1.games});
        let weapon_stats_formatted=weapon_stats[..usize::min(5,weapon_stats.len())].iter().fold(String::new(), |acc,stat|{
            format!("{acc}\n{:20}{}",stat.0,stat.1)
        });
        let mut mode_stats:Vec<(&Mode,&StatBreakdown)>=self.mode_stats.iter().collect();
        mode_stats.sort_by_key(|stat|{stat.1.games});
        let mode_stats_formatted=mode_stats.iter().fold(String::new(), |acc,stat|{
            format!("{acc}\n{:20}{}",stat.0.to_string(),stat.1)
        });
        format!("```                    {HEADER}\nTotal               {}\nToday               {}\n\nWeapons             {HEADER}{weapon_stats_formatted}\n\nMode                {HEADER}{mode_stats_formatted}```",self.total_stats,self.todays_stats)
    }

    fn check_for_old_day_stats(&mut self){
        if self.stats_date!=chrono::Local::now().date_naive(){
            self.todays_stats=StatBreakdown::default();
            self.stats_date=chrono::Local::now().date_naive();
        }
    }
}

#[derive(Deserialize,Serialize,Debug,Default)]
struct StatBreakdown{
    games:u32,
    wins:u32,
    losses:u32,
    kills:u32,
    assists:u32,
    deaths:u32,
    specials:u32,
    points:u32,
}

impl Display for StatBreakdown{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:5}  {:4.1}%  {:5.2} {:5.2} {:5.2} {:5.2} {:5.2}",self.games,f32::round((self.wins as f32 / self.games as f32)*100.0), self.kills as f32 / (self.games as f32) ,self.assists as f32 / (self.games as f32), self.deaths as f32 / (self.games as f32),self.specials as f32/self.games as f32, self.kills as f32/self.deaths as f32)
    }
}

impl AddAssign<&Player> for StatBreakdown{
    fn add_assign(&mut self, player: &Player) {
        match player.battle_result{
            Some(BattleResult::Win)=>{self.wins+=1;},
            Some(BattleResult::Lose)=>{self.losses+=1},
            _=>()
        }
        self.games+=1;
        self.kills+=player.kills as u32;
        self.assists+=player.assists as u32;
        self.deaths+=player.deaths as u32;
        self.specials+=player.specials as u32;
        self.points+=player.turf_inked as u32;
    }
}

impl From<&Player> for StatBreakdown{
    fn from(player: &Player) -> Self {
        Self { games: 1, wins: (player.battle_result==Some(BattleResult::Win)) as u32, losses:(player.battle_result==Some(BattleResult::Lose)) as u32, kills: player.kills as u32, assists: player.assists as u32, deaths: player.deaths as u32, specials: player.specials as u32, points: player.turf_inked as u32}
    }
}

pub fn get_tracked_stats()->anyhow::Result<Option<HashMap<String,TotalPlayerStats>>>{
    // let stats_path:&Path=Path::new(STATS_PATH);
    if fs::exists(STATS_PATH)?{
        anyhow::ensure!(fs::metadata(STATS_PATH)?.is_file(),"stats.json cant be a folder");
        let stats_file=File::open(STATS_PATH)?;
        Ok(Some(serde_json::from_reader(stats_file)?))
    }else{
        Ok(None)
    }
}

pub fn write_tracked_stats(stats:&HashMap<String,TotalPlayerStats>)->anyhow::Result<()>{
    // let stats_path:&Path=Path::new(STATS_PATH);
    let file=File::create(STATS_PATH)?;
    Ok(serde_json::to_writer_pretty(file, stats)?)
}