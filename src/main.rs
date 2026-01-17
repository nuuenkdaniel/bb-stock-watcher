use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use dotenv::dotenv;
use anyhow::{Context, Result, bail};

use std::env;

#[derive(Deserialize, Debug)]
struct QueryResult {
    total: i64,
    products: Vec<Product>
}

#[derive(Deserialize, Debug)]
struct Product {
    sku: i64,
    name: String,
    #[allow(non_snake_case)]
    onlineAvailability: bool, 
    #[allow(non_snake_case)]
    inStoreAvailability: bool,
    orderable: String,
    active: bool
}

struct BestBuyCalls {
    api_key: String,
    client: Client,
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

#[tokio::main]
async fn main() -> Result<()>{
    dotenv().ok();
    let api_key: String = env::var("BEST_BUY_KEY").expect("BEST_BUY_KEY not found");

    let best_buy = BestBuyCalls {
        api_key: api_key,
        client: Client::new()
    };

    let skus = vec!["6550199"];
    let result: QueryResult = best_buy.get_skus_details(skus).await?;
    dbg!(result);
    Ok(())
}
