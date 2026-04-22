use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use std::collections::HashMap;
use std::ops::AddAssign;
use crate::battle::*;
// use std::path::Path;
use tokio::{fs as tfs, io::AsyncReadExt};
use std::sync::{Arc, Mutex};
use std::fs::{File,self};
use anyhow;
use serde_json;

const STATS_PATH:&str="stats.json";

#[derive(Serialize,Deserialize,Default,Debug)]
pub struct AllStats{
    player_stats:HashMap<String,TotalPlayerStats>,
    wins:u32,
    losses:u32,
    time_spent:u32,
}

impl AllStats{
    pub fn add_game(&mut self, battle: &Battle, tracked_players:&Vec<String>) {
        match battle.result{
            BattleResult::Win=>{self.wins+=1;},
            BattleResult::Lose=>{self.losses+=1;},
            _=>()
        }
        for player in &battle.our_players{
            if tracked_players.contains(&player.name){
                //player is one of the tracked player, add stats
                self.player_stats.entry(player.name.clone()).or_insert(TotalPlayerStats::default()).add_game(player, battle.mode);
                if let Err(err)=write_tracked_stats(self){dbg!(err);}
            }
        }
        self.time_spent+=battle.duration as u32;
    }
    pub async fn from_past_games(results_dir: &str, tracked_players:Vec<String>)->anyhow::Result<Self>{
        let stats=Arc::new(Mutex::new(Self::default()));
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
                            stats.add_game(&battle, &tracked_players);
                        }
                    }
                });
            }
        }
        tasks.join_all().await;
        Ok(Arc::into_inner(stats).expect("not all stats instances were dropped").into_inner()?)
    }
}


#[derive(Deserialize,Serialize,Default,Debug)]
struct TotalPlayerStats{
    total_stats:StatBreakdown,
    weapon_stats:HashMap<String,StatBreakdown>,
    mode_stats:HashMap<Mode,StatBreakdown>
}

impl TotalPlayerStats{
    fn add_game(&mut self,player:&Player,mode:Mode){
        self.total_stats+=player;
        self.weapon_stats.entry(player.weapon.clone()).and_modify(|weapon_stat|{*weapon_stat+=player;}).or_insert(StatBreakdown::from(player));
        self.mode_stats.entry(mode).and_modify(|mode_stat|{*mode_stat+=player;}).or_insert(StatBreakdown::from(player));
    }
}

#[derive(Deserialize,Serialize,Debug)]
struct StatBreakdown{
    games:u32,
    kills:u32,
    deaths:u32,
    specials:u32,
    points:u32,
}

impl Default for StatBreakdown {
    fn default() -> Self {
        StatBreakdown { games: 0, kills: 0, deaths: 0, specials: 0, points: 0}
    }
}

impl AddAssign<&Player> for StatBreakdown{
    fn add_assign(&mut self, rhs: &Player) {
        self.games+=1;
        self.kills+=rhs.kills as u32;
        self.deaths+=rhs.deaths as u32;
        self.specials+=rhs.specials as u32;
        self.points+=rhs.turf_inked as u32;
    }
}

impl From<&Player> for StatBreakdown{
    fn from(value: &Player) -> Self {
        Self { games: 1, kills: value.kills as u32, deaths: value.deaths as u32, specials: value.specials as u32, points: value.turf_inked as u32}
    }
}

pub fn get_tracked_stats()->anyhow::Result<AllStats>{
    // let stats_path:&Path=Path::new(STATS_PATH);
    if fs::exists(STATS_PATH)?{
        anyhow::ensure!(fs::metadata(STATS_PATH)?.is_file(),"stats.json cant be a folder");
        let stats_file=File::open(STATS_PATH)?;
        Ok(serde_json::from_reader(stats_file)?)
    }else{
        Ok(AllStats::default())
    }
}

pub fn write_tracked_stats(stats:&AllStats)->anyhow::Result<()>{
    // let stats_path:&Path=Path::new(STATS_PATH);
    let file=File::create(STATS_PATH)?;
    Ok(serde_json::to_writer_pretty(file, stats)?)
}