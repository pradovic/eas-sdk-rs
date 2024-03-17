use eas_sdk_rs::client;
use eas_sdk_rs::eas::AttestationDataBuilder;
use ethers::abi::Token;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::utils::hex;
use std::error::Error;
use std::str::FromStr;

const SEPOLIA_CONTRACT_ADDRESS: &str = "0xC2679fBD37d54388Ce493F1DB75320D236e1815e";
const SEPOLIA_ENDPOINT: &str = "https://ethereum-sepolia-rpc.publicnode.com/";
const SEPOLIA_PRIVATE_KEY_HEX: &str = "enter your private key here";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let singing_key = &hex::decode(SEPOLIA_PRIVATE_KEY_HEX)?;
    let signing_key = SigningKey::from_slice(singing_key)?;
    let contract_address = H160::from_str(SEPOLIA_CONTRACT_ADDRESS)?;
    let client = client::Client::new(SEPOLIA_ENDPOINT, contract_address, None, signing_key).await?;

    let schema_id =
        H256::from_str("0x0cf8b15034904dc56810da6237d220c345693077c4d42c53c45263649ed3b585")?;

    let attestation_request = AttestationDataBuilder::new()
        .revocable(true)
        .data(&[
            Token::Address(H160::from_low_u64_be(0x1234567890abcdef)),
            Token::Address(H160::from_low_u64_be(0x1234567890abcdef)),
            Token::Int(11.into()),
        ])
        .build();

    // attest sends a transaction and returns a PendingTx that can be awaited
    let tx = client.eas.attest(&schema_id, attestation_request).await?;
    let attested = tx.await?;

    println!("Attestation ID: {:?}", attested);

    // timestamp send a transaction and returns a PendingTx that can be awaited
    let tx = client.eas.timestamp(attested.uid).await?;
    let timestamped = tx.await?;

    println!("Timestamped: {:?}", timestamped);

    // revoke send a transaction and returns a PendingTx that can be awaited
    let revoked = client
        .eas
        .revoke(attested.schema, attested.uid, U256::default())
        .await?;

    let revoked = revoked.await?;

    println!("Revoked: {:?}", revoked);

    Ok(())
}
