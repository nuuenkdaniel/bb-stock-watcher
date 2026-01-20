use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use dotenv::dotenv;
use anyhow::{Context, Result};
use tokio::time::{self, Duration};

use std::env;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct QueryResult {
    // total: i64,
    products: Vec<Product>
}

#[derive(Deserialize, Debug)]
struct Product {
    sku: i64,
    name: String,
    #[serde(rename = "onlineAvailability")]
    online_availability: bool, 
    #[serde(rename = "inStoreAvailability")]
    in_store_availability: bool,
    // orderable: String,
    //active: bool
}

struct BestBuyCalls {
    api_key: String,
    client: Client,
}

struct GotifyNotif {
    api_key: String,
    client: Client,
    server: String
}

impl BestBuyCalls {
    async fn get_skus_details(
        &self,
        skus: Vec<&str>
    ) -> Result<QueryResult> {
        let skus_formatted = skus.join(",");
        let url = format!(
            "https://api.bestbuy.com/v1/products(sku%20in({}))?show=sku,name,onlineAvailability,inStoreAvailability,orderable,active&apiKey={}&format=json",
            skus_formatted,
            self.api_key
        );
        self.client
            .get(url)
            .send()
            .await
            .context("Failed to connect to best buy api")?
            .json::<QueryResult>()
            .await
            .context("Failed to obtain json for products")
    }
}

impl GotifyNotif {
    async fn send_notif(
        &self,
        title: &str,
        message: &String,
        priority: u32
    ) -> Result<()> {
        let url = format!("https://{}/message?token={}", self.server, self.api_key);
        self.client
            .post(url)
            .json(&json!({
                "title": title,
                "message": message,
                "priority": priority
            }))
        .send()
            .await
            .context("Failed to connect to gotify server")?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()>{
    dotenv().ok();

    // Get env vars
    let bb_api_key: String = env::var("BEST_BUY_KEY").expect("BEST_BUY_KEY not found");
    let skus_raw = env::var("SKUS").expect("SKUS not found");
    let repeat_raw = env::var("REPEAT").unwrap_or_default();
    let interval_raw = env::var("INTERVAL").unwrap_or_default();
    let gotify_status_raw = env::var("GOTIFY").unwrap_or_default();
    let gotify_api_key: String = env::var("GOTIFY_API_KEY").expect("GOTIFY_API_KEY not found");
    let gotify_server: String = env::var("GOTIFY_SERVER").expect("GOTIFY_SERVER not found");
    let gotify_priority_raw = env::var("GOTIFY_PRIORITY").unwrap_or_default();

    // Reformat env vars
    let skus: Vec<&str> = skus_raw
        .split(",")
        .map(|s| s.trim())
        .collect();
    let repeat = match repeat_raw.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        _ => false
    };
    let gotify_status = match gotify_status_raw.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        _ => false
    };
    let interval: u64 = interval_raw.parse().unwrap_or(300);
    let gotify_priority: u32 = gotify_priority_raw.parse().unwrap_or(0);

    // Initialize structs
    let client: Client = Client::new();
    let best_buy = BestBuyCalls {
        api_key: bb_api_key,
        client: client.clone()
    };
    let gotify_notif = GotifyNotif {
        api_key: gotify_api_key,
        client: client.clone(),
        server: gotify_server
    };

    let mut timer = time::interval(Duration::from_secs(interval));
    let mut product_status_map: HashMap<String,bool> = HashMap::new();
    let _ = skus.iter()
        .map(|&sku| product_status_map.insert(sku.to_string(), false));
    loop {
        if repeat { timer.tick().await; }
        let resp = best_buy.get_skus_details(skus.clone()).await;
        if let Err(e) = &resp {
            eprintln!("Error querying availability: {}", e);
            continue;
        }
        let result = resp?;
        println!("Queried");
        for product in result.products {
            // dbg!(&product);
            let product_sku = product.sku.to_string();
            if product.in_store_availability || product.online_availability {
                let product_status = product_status_map.insert(product_sku, true);
                if !product_status.unwrap_or(false) {
                    let message = format!("Sku: {} aka \"{}\" is available", product.sku, product.name);
                    if gotify_status {
                        let notif_title = "Product Available";
                        let resp = gotify_notif.send_notif(&notif_title, &message, gotify_priority).await;
                        if let Err(e) = &resp {
                            eprintln!("Error reaching gotify server: {}", e);
                            continue;
                        }
                    }
                    println!("{}", message);
                }
            }
            else {
                let product_status = product_status_map.insert(product_sku, false);
                if product_status.unwrap_or(false) {
                    let message = format!("Sku: {} aka \"{}\" is no longer available", product.sku, product.name);
                    if gotify_status {
                        let notif_title = "Product Unavailable";
                        let resp = gotify_notif.send_notif(&notif_title, &message, gotify_priority).await;
                        if let Err(e) = &resp {
                            eprintln!("Error reaching gotify server: {}", e);
                            continue;
                        }
                    }
                    println!("{}", message);
                }
            }
        }
    }
}
