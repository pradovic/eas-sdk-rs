use std::sync::Arc;

use crate::contracts::eas::TimestampedFilter;
use ethers::{
    contract::{EthLogDecode, Event},
    providers::{Middleware, PendingTransaction, PubsubClient},
    types::{BlockNumber, H256},
};
use futures::{stream::BoxStream, Stream, StreamExt};
use tracing::debug;

use crate::transaction::{Error as TxError, PendingTx};

use super::{Error, EAS};

pub type Timestamp = u64;

#[derive(Debug)]
pub struct Timestamped {
    pub data: H256,
    pub timestamp: u64,
}

impl From<TimestampedFilter> for Timestamped {
    fn from(event: TimestampedFilter) -> Self {
        Self {
            data: H256::from_slice(&event.data),
            timestamp: event.timestamp,
        }
    }
}

impl<M: Middleware + 'static> EAS<M> {
    pub async fn get_timestamped(&self, data: H256) -> Result<Timestamp, Error<M>> {
        let binding = self.contract.get_timestamp(data.into());
        let timestamp = binding.call().await?;
        Ok(timestamp)
    }

    pub async fn timestamp(&self, data: H256) -> Result<PendingTx<Timestamped>, Error<M>> {
        let binding = self.contract.timestamp(data.into());

        let tx = binding.send().await?;
        debug!("Transaction sent: {:?}", tx.tx_hash());

        // PendingTransaction is not copy and it is dropped there, but it is recreated from tx_hash and provider
        let tx_hash = tx.tx_hash();
        let provider = self.contract.client().clone();

        let future = async move {
            let tx = PendingTransaction::new(tx_hash, provider.provider());
            let receipt = tx.await?;
            let receipt = receipt.ok_or_else(|| TxError::TransactionDropped(tx_hash))?;

            debug!("Receipt: {:?}", receipt);

            let log = receipt.logs.first().ok_or_else(|| TxError::MissingLogs)?;

            let timestamped = TimestampedFilter::decode_log(&log.clone().into())?;

            Ok(timestamped.into())
        };

        Ok(PendingTx::new(future))
    }

    pub async fn multi_timestamp(
        &self,
        data: Vec<H256>,
    ) -> Result<PendingTx<Vec<Result<Timestamped, TxError>>>, Error<M>> {
        let binding = self
            .contract
            .multi_timestamp(data.into_iter().map(|d| d.into()).collect());

        let tx = binding.send().await?;
        debug!("Transaction sent: {:?}", tx.tx_hash());

        // PendingTransaction is not copy and it is dropped there, but it is recreated from tx_hash and provider
        let tx_hash = tx.tx_hash();
        let provider = self.contract.client().clone();

        let future = async move {
            let tx = PendingTransaction::new(tx_hash, provider.provider());
            let receipt = tx.await?;
            let receipt = receipt.ok_or_else(|| TxError::TransactionDropped(tx_hash))?;

            debug!("Receipt: {:?}", receipt);

            let timestamped: Vec<Result<Timestamped, TxError>> = receipt
                .logs
                .iter()
                .map(|log| {
                    TimestampedFilter::decode_log(&log.clone().into())
                        .map(Into::into)
                        .map_err(TxError::from)
                })
                .collect();

            Ok(timestamped)
        };

        Ok(PendingTx::new(future))
    }

    pub fn timestamped_event<T>(
        &self,
        start_block: T,
        end_block: Option<T>,
        uids: Option<Vec<H256>>,
    ) -> TimestampedEvent<M>
    where
        T: Into<BlockNumber>,
    {
        let mut event = self.contract.timestamped_filter().from_block(start_block);

        if let Some(end_block) = end_block {
            event = event.to_block(end_block);
        }

        if let Some(uids) = uids {
            event = event.topic0(uids);
        }

        TimestampedEvent::new(event)
    }
}

pub struct TimestampedEvent<M: Middleware + 'static> {
    inner: Event<Arc<M>, M, TimestampedFilter>,
}

impl<M: Middleware> TimestampedEvent<M> {
    pub fn new(filter: Event<Arc<M>, M, TimestampedFilter>) -> Self {
        Self { inner: filter }
    }

    pub async fn stream(&self) -> Result<BoxStream<Result<Timestamped, Error<M>>>, Error<M>> {
        let stream = self
            .inner
            .stream()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Timestamped::from(e?)));
        Ok(stream.boxed())
    }

    pub async fn query(&self) -> Result<Vec<Timestamped>, Error<M>> {
        let logs = self
            .inner
            .query()
            .await
            .map_err(Error::ContractError)?
            .iter()
            .map(|e| Timestamped::from(e.clone()))
            .collect();

        Ok(logs)
    }
}

impl<M: Middleware> TimestampedEvent<M>
where
    M::Provider: PubsubClient,
{
    pub async fn subscribe(
        &self,
    ) -> Result<impl Stream<Item = Result<Timestamped, Error<M>>> + '_, Error<M>> {
        let stream = self
            .inner
            .subscribe()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Timestamped::from(e?)));
        Ok(stream)
    }
}
