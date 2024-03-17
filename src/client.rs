use std::sync::Arc;

use ethers::providers::Middleware;

use crate::eas::EAS;
use crate::schema_registry::SchemaRegistry;

use crate::contracts::eas::EAS as EasContract;
use crate::contracts::schema_registry::SchemaRegistry as SchemaRegistryContract;

use ethers::core::k256::ecdsa::SigningKey;

use ethers::prelude::*;

pub struct Client<M: Middleware> {
    pub eas: EAS<M>,
    pub schema_registry: SchemaRegistry<M>,
}

impl<M: Middleware + 'static> Client<M> {
    pub async fn new_with_middleware<T: Into<::ethers::core::types::Address>>(
        eas_contract_address: T,
        middlewhare: Arc<M>,
        schema_contract_address: Option<T>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let eas_contract = EasContract::new(eas_contract_address, middlewhare.clone());

        let schema = match schema_contract_address {
            Some(address) => address.into(),
            None => eas_contract.get_schema_registry().call().await?,
        };

        let schema = SchemaRegistryContract::new(schema, middlewhare.clone());

        Ok(Self {
            eas: EAS::new(eas_contract),
            schema_registry: SchemaRegistry::new(schema),
        })
    }
}

impl Client<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>> {
    pub async fn new<T: Into<::ethers::core::types::Address>>(
        endpoint: &str,
        eas_contract_address: T,
        schema_contract_address: Option<T>,
        signing_key: SigningKey,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(endpoint)?;
        let chain_id = provider.get_chainid().await?.as_u64();

        let wallet = Wallet::from(signing_key).with_chain_id(chain_id);
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        Client::new_with_middleware(eas_contract_address, client, schema_contract_address).await
    }
}
