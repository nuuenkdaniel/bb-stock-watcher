use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use dotenv::dotenv;
use anyhow::{Context, Result};

use std::env;

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
        let url = format!("https://api.bestbuy.com/v1/products(sku%20in({}))?show=sku,name,onlineAvailability,inStoreAvailability,orderable,active&apiKey={}&format=json", skus_formatted, self.api_key);
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
        priority: i32
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
        let bb_api_key: String = env::var("BEST_BUY_KEY").expect("BEST_BUY_KEY not found");
        let gotify_api_key: String = env::var("GOTIFY_API_KEY").expect("GOTIFY_API_KEY not found");
        let gotify_server: String = env::var("GOTIFY_SERVER").expect("GOTIFY_SERVER not found");

        let client: Client = Client::new();

        let best_buy = BestBuyCalls {
            api_key: bb_api_key,
            client: client.clone()
        };

        let skus = vec!["6550199"];
        let result: QueryResult = best_buy.get_skus_details(skus).await?;
        dbg!(&result);
        let gotify_notif = GotifyNotif {
            api_key: gotify_api_key,
            client: client.clone(),
            server: gotify_server
        };
        let title = "Product available";
        let priority = 32;
        for product in result.products {
            dbg!(&product);
            if product.in_store_availability == true || product.online_availability == true {
                let message = format!("Sku: {} aka \"{}\" is available", product.sku, product.name);
                gotify_notif.send_notif(&title, &message, priority).await?;
            }
        }
        Ok(())
    }
