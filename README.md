This repository contains the unofficial & opinionated Ethereum Attestation Service SDK for the Rust programming language, used to interact with the Ethereum Attestation Service Protocol.

[Ethereum Attestation Service](https://attest.sh/) (EAS) is an open-source infrastructure public good for making attestations onchain or offchain.

Rust SDK interacts with [EAS Smart Contracts](https://github.com/ethereum-attestation-service/eas-contracts) deployed on different EVM-compatible blockchains using smart contract bindings. The list off deployed contracts could be found in [EAS Contracts README file](https://github.com/ethereum-attestation-service/eas-contracts?tab=readme-ov-file#deployments). EAS contract address for a desired network in that list should be passed as an argument to the client constructor.


## Usage

The async client uses `ethers-rs`  as dependencies. To use it you would need to setup your `Cargo.toml` to something like this:

```toml
[dependencies]
ethers = "2.0"
eas-sdk = "0.1.0"
```

And then use it in your code:

```rust
const SEPOLIA_CONTRACT_ADDRESS: &str = "0xC2679fBD37d54388Ce493F1DB75320D236e1815e";
const SEPOLIA_ENDPOINT: &str = "https://ethereum-sepolia-rpc.publicnode.com/";
const SEPOLIA_PRIVATE_KEY_HEX: &str = "enter your private key here";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let singing_key = &hex::decode(SEPOLIA_PRIVATE_KEY_HEX)?;
    let signing_key = SigningKey::from_slice(singing_key)?;
    let contract_address = H160::from_str(SEPOLIA_CONTRACT_ADDRESS)?;
    let client = client::Client::new(SEPOLIA_ENDPOINT, contract_address, None, signing_key).await?;

    // Register a new schema
    let schema = schema::SchemaBuilder::new()
        .add("id", schema::Type::Address)
        .add("voter_id", schema::Type::Address)
        .add("vote_option", schema::Type::Int(32))
        .build();

    let tx = client.schema_registry.register(schema, None, true).await?;
    let registered_schema = tx.await?;
    println!("Schema ID: {:?}", registered_schema.schema_uid);

    // Make a new attestation for the created schema
    let attestation_request = AttestationDataBuilder::new()
        .revocable(true)
        .data(&[
            Token::Address(H160::from_low_u64_be(0x1234567890abcdef)),
            Token::Address(H160::from_low_u64_be(0x1234567890abcdef)),
            Token::Int(11.into()),
        ])
        .build();

    // attest sends a transaction and returns a PendingTx that can be awaited
    let tx = client.eas.attest(&registered_schema.schema_uid, attestation_request).await?;
    let attested = tx.await?;

    println!("Attestation ID: {:?}", attested);

    Ok(())
}
```

Both `schema_registry` and `eas` types export underlying contracts as well. These can be used to interact with abigen bindings directly, if needed. Feel free to check more detailed usage examples in the `/examples` folder.

## Status
This project is created from a need to use Ethereum Attestation Service from Rust in my personal projects. As the EAS is still evolving project, I expect for this library to evolve as well. 

The project is still in early phase. The on-chain flow is covered, but there are still missing features, such as offchain attestations, for example. It is pretty opinionated as I was making it to suit my own needs, but I tried to keep it pretty general as well. If you happen to be using the SDK and find missing feature, feel free to reach out or open an issue, or even contribute :)


## Versioning

Each version of the client is tagged and the version is updated accordingly.
To see the list of past versions, run `git tag`.


## License

This library is distributed under the BSD-style license found in the [LICENSE](LICENSE) file.

