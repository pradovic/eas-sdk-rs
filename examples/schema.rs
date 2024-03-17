use eas_sdk_rs::eas::AttestationDataBuilder;
use eas_sdk_rs::{client, schema};
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

    let schema = schema::SchemaBuilder::new()
        .add("id", schema::Type::Address)
        .add("voter_id", schema::Type::Address)
        .add("vote_option", schema::Type::Int(32))
        .build();

    let tx = client.schema_registry.register(schema, None, true).await?;
    let registered_schema = tx.await?;

    println!("Schema ID: {:?}", registered_schema.schema_uid);

    let naming_schema =
        H256::from_str("0x44d562ac1d7cd77e232978687fea027ace48f719cf1d58c7888e509663bb87fc")?;

    let data = &[
        Token::FixedBytes(registered_schema.schema_uid.as_bytes().into()),
        Token::String("enter schema name here".to_string()),
    ];

    let attestation_data = AttestationDataBuilder::new()
        .revocable(true)
        .data(data)
        .build();

    let attested = client.eas.attest(&naming_schema, attestation_data).await?;
    let attested = attested.await?;

    println!("Attestation ID: {:?}", attested);

    Ok(())
}
