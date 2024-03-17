use std::sync::Arc;

use ethers::{
    contract::{ContractError, EthLogDecode, Event},
    core::types::Address,
    providers::{Middleware, PendingTransaction, PubsubClient},
    types::{BlockNumber, H256},
};

use crate::contracts::{
    eas::SchemaRecord,
    schema_registry::RegisteredFilter,
    schema_registry::{AlreadyExists, SchemaRegistry as Contract},
};

use thiserror::Error;

use futures::{stream::BoxStream, Stream, StreamExt};
use tracing::debug;

use crate::transaction::{Error as TxError, PendingTx};

#[derive(Error, Debug)]
pub enum SchemaRegistryError<M: Middleware> {
    #[error("schema already exists")]
    AlreadyExists,

    #[error("contract error: {0}")]
    ContractError(#[from] ContractError<M>),

    #[error("unknown error: {0}")]
    Other(String),
}

#[derive(Debug)]
pub struct SchemaRegistryRegistered {
    pub schema_uid: H256,
    pub registerer: Address,
}

#[derive(Debug)]
pub struct Schema {
    pub uid: H256,
    pub resolver: Address,
    pub revocable: bool,
    pub schema: String,
}

impl From<SchemaRecord> for Schema {
    fn from(record: SchemaRecord) -> Self {
        Self {
            uid: H256::from(&record.uid),
            resolver: record.resolver,
            revocable: record.revocable,
            schema: record.schema,
        }
    }
}

impl From<RegisteredFilter> for SchemaRegistryRegistered {
    fn from(filter: RegisteredFilter) -> Self {
        Self {
            schema_uid: H256::from(&filter.uid),
            registerer: filter.registerer,
        }
    }
}
pub struct SchemaRegistry<M: Middleware> {
    contract: Contract<M>,
}

impl<M: Middleware + 'static> SchemaRegistry<M> {
    pub fn new(contract: Contract<M>) -> Self {
        Self { contract }
    }

    pub async fn register<T: Into<String>>(
        &self,
        schema: T,
        resolver: Option<Address>,
        revocable: bool,
    ) -> Result<PendingTx<SchemaRegistryRegistered>, SchemaRegistryError<M>> {
        let resolver = resolver.unwrap_or_else(Address::zero);
        let binding = self.contract.register(schema.into(), resolver, revocable);

        let tx = match binding.send().await {
            Ok(tx) => tx,
            Err(e) => {
                if e.decode_revert::<AlreadyExists>().is_some() {
                    return Err(SchemaRegistryError::AlreadyExists);
                }

                return Err(SchemaRegistryError::ContractError(e));
            }
        };

        debug!("Transaction sent: {:?}", tx.tx_hash());

        // PendingTransaction is not copy and it is dropped there, but it is recreated from tx_hash and provider
        let tx_hash = tx.tx_hash();
        let provider = self.contract.client().clone();

        let future = async move {
            let tx = PendingTransaction::new(tx_hash, provider.provider());
            let receipt = tx.await.map_err(TxError::Provider)?;
            let receipt = receipt.ok_or_else(|| TxError::TransactionDropped(tx_hash))?;

            debug!("Receipt: {:?}", receipt);

            let log = receipt.logs.first().ok_or_else(|| TxError::MissingLogs)?;
            let registered_event = RegisteredFilter::decode_log(&log.clone().into())?;

            Ok(SchemaRegistryRegistered::from(registered_event))
        };

        Ok(PendingTx::new(future))
    }

    pub async fn get_schema(&self, schema_uid: H256) -> Result<Schema, SchemaRegistryError<M>> {
        let binding = self.contract.get_schema(schema_uid.into());

        // todo: handle not found
        let schema = binding
            .call()
            .await
            .map_err(SchemaRegistryError::ContractError)?;

        Ok(schema.into())
    }

    pub async fn version(&self) -> Result<String, SchemaRegistryError<M>> {
        let binding = self.contract.version();
        let version = binding
            .call()
            .await
            .map_err(SchemaRegistryError::ContractError)?;

        Ok(version)
    }

    pub fn registered_event<T>(
        &self,
        start_block: T,
        end_block: Option<T>,
        uids: Option<Vec<H256>>,
    ) -> SchemaRegisteredEvent<M>
    where
        T: Into<BlockNumber>,
    {
        let mut event = self.contract.registered_filter().from_block(start_block);

        if let Some(end_block) = end_block {
            event = event.to_block(end_block);
        }

        if let Some(uids) = uids {
            event = event.topic0(uids);
        }

        SchemaRegisteredEvent::new(event)
    }

    pub fn contract(&self) -> &Contract<M> {
        &self.contract
    }
}

pub struct SchemaRegisteredEvent<M: Middleware + 'static> {
    inner: Event<Arc<M>, M, RegisteredFilter>,
}

impl<M: Middleware> SchemaRegisteredEvent<M> {
    pub fn new(filter: Event<Arc<M>, M, RegisteredFilter>) -> Self {
        Self { inner: filter }
    }

    pub async fn stream(
        &self,
    ) -> Result<
        BoxStream<Result<SchemaRegistryRegistered, SchemaRegistryError<M>>>,
        SchemaRegistryError<M>,
    > {
        let stream = self
            .inner
            .stream()
            .await
            .map_err(SchemaRegistryError::ContractError)?
            .map(|e| Ok(SchemaRegistryRegistered::from(e?)));
        Ok(stream.boxed())
    }

    pub async fn query(&self) -> Result<Vec<SchemaRegistryRegistered>, SchemaRegistryError<M>> {
        let logs = self
            .inner
            .query()
            .await
            .map_err(SchemaRegistryError::ContractError)?
            .iter()
            .map(|e| SchemaRegistryRegistered::from(e.clone()))
            .collect();

        Ok(logs)
    }
}

impl<M: Middleware> SchemaRegisteredEvent<M>
where
    M::Provider: PubsubClient,
{
    pub async fn subscribe(
        &self,
    ) -> Result<
        impl Stream<Item = Result<SchemaRegistryRegistered, SchemaRegistryError<M>>> + '_,
        SchemaRegistryError<M>,
    > {
        let stream = self
            .inner
            .subscribe()
            .await
            .map_err(SchemaRegistryError::ContractError)?
            .map(|e| Ok(SchemaRegistryRegistered::from(e?)));
        Ok(stream)
    }
}
