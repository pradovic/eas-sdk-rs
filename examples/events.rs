use eas_sdk_rs::client;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::utils::hex;
use std::error::Error;
use std::str::FromStr;

const SEPOLIA_CONTRACT_ADDRESS: &str = "0xC2679fBD37d54388Ce493F1DB75320D236e1815e";
const SEPOLIA_PRIVATE_KEY_HEX: &str = "enter your private key here";
const SEPOLIA_ENDPOINT: &str = "https://eth-sepolia.g.alchemy.com/v2/{your-api-key}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let singing_key = &hex::decode(SEPOLIA_PRIVATE_KEY_HEX)?;
    let signing_key = SigningKey::from_slice(singing_key)?;
    let contract_address = H160::from_str(SEPOLIA_CONTRACT_ADDRESS)?;
    let client = client::Client::new(SEPOLIA_ENDPOINT, contract_address, None, signing_key).await?;

    // Filter for the events starting with the block ID 5553102
    let filter = client.eas.attested_event(5553102, None, None);

    // Get the query for the filter
    // This will return the logs that match the filter all at once
    let query = filter.query().await?;
    println!("Query: {:?}", query);

    // Get the stream for the filter
    let mut stream = filter.stream().await.expect("Failed to get stream");

    // This will print the logs as they come in
    while let Some(Ok(approval)) = stream.next().await {
        println!("Approval: {:?}", approval);
    }

    Ok(())
}
