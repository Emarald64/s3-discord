use tokio;
use reqwest::{Client,header};
use serde_json::{Map, Value};

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
                    dbg!(battles);
                    // for battle_id in battles{
                    //     if let Value::String(battle_id)=battle_id{
                            
                    //     }
                    // }
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
