use std::sync::Arc;

use crate::contracts::eas::{
    MultiRevocationRequest, RevocationRequest, RevocationRequestData, RevokedFilter,
};
use crate::transaction::{Error as TxError, PendingTx};
use ethers::{
    contract::{EthLogDecode, Event},
    providers::{Middleware, PendingTransaction, PubsubClient},
    types::{Address, BlockNumber, H256, U256},
};
use futures::{stream::BoxStream, Stream, StreamExt};
use tracing::debug;

use super::{Error, EAS};

#[derive(Debug)]
pub struct Revoked {
    pub recipient: Address,
    pub attester: Address,
    pub uid: H256,
    pub schema: H256,
}

pub struct MultiRevokeRequest {
    pub schema_id: H256,
    pub data: Vec<RevokeRequestData>,
}

pub struct RevokeRequestData {
    pub attestation_id: H256,
    pub value: U256,
}

impl From<MultiRevokeRequest> for MultiRevocationRequest {
    fn from(req: MultiRevokeRequest) -> Self {
        Self {
            schema: req.schema_id.into(),
            data: req.data.into_iter().map(|data| data.into()).collect(),
        }
    }
}

impl From<RevokeRequestData> for RevocationRequestData {
    fn from(data: RevokeRequestData) -> Self {
        Self {
            uid: data.attestation_id.into(),
            value: data.value,
        }
    }
}

impl From<RevokedFilter> for Revoked {
    fn from(event: RevokedFilter) -> Self {
        Self {
            recipient: event.recipient,
            attester: event.attester,
            uid: H256::from_slice(&event.uid),
            schema: H256::from_slice(&event.schema),
        }
    }
}

impl<M: Middleware + 'static> EAS<M> {
    pub async fn revoke(
        &self,
        schema_id: H256,
        attestation_id: H256,
        value: U256,
    ) -> Result<PendingTx<Revoked>, Error<M>> {
        let req = RevocationRequest {
            schema: schema_id.into(),
            data: RevocationRequestData {
                uid: attestation_id.into(),
                value,
            },
        };

        let binding = self.contract.revoke(req);
        let tx = binding.send().await?;

        debug!("Transaction sent: {:?}", tx.tx_hash());

        // PendingTransaction is not copy and it is dropped there, but it is recreated from tx_hash and provider
        let tx_hash = tx.tx_hash();
        let provider = self.contract.client().clone();

        let future = async move {
            let tx = PendingTransaction::new(tx_hash, provider.provider());
            let receipt = tx.await?;
            let receipt = receipt.ok_or_else(|| TxError::TransactionDropped(tx_hash))?;

            let log = receipt.logs.first().ok_or_else(|| TxError::MissingLogs)?;

            let revoked = RevokedFilter::decode_log(&log.clone().into())?;
            Ok(revoked.into())
        };

        Ok(PendingTx::new(future))
    }

    pub async fn multi_revoke(
        &self,
        revokes: Vec<MultiRevokeRequest>,
    ) -> Result<PendingTx<Vec<Result<Revoked, TxError>>>, Error<M>> {
        let requests: Vec<MultiRevocationRequest> =
            revokes.into_iter().map(|req| req.into()).collect();

        let binding = self.contract.multi_revoke(requests);
        let tx = binding.send().await?;

        debug!("Transaction sent: {:?}", tx.tx_hash());

        // PendingTransaction is not copy and it is dropped there, but it is recreated from tx_hash and provider
        let tx_hash = tx.tx_hash();
        let provider = self.contract.client().clone();

        let future = async move {
            let tx = PendingTransaction::new(tx_hash, provider.provider());
            let receipt = tx.await?;
            let receipt = receipt.ok_or_else(|| TxError::TransactionDropped(tx_hash))?;

            let revoked: Vec<Result<Revoked, TxError>> = receipt
                .logs
                .iter()
                .map(|log| {
                    RevokedFilter::decode_log(&log.clone().into())
                        .map(Into::into)
                        .map_err(TxError::from)
                })
                .collect();

            Ok(revoked)
        };

        Ok(PendingTx::new(future))
    }

    pub fn revoked_event<T>(
        &self,
        start_block: T,
        end_block: Option<T>,
        uids: Option<Vec<H256>>,
    ) -> RevokedEvent<M>
    where
        T: Into<BlockNumber>,
    {
        let mut event = self.contract.revoked_filter().from_block(start_block);

        if let Some(end_block) = end_block {
            event = event.to_block(end_block);
        }

        if let Some(uids) = uids {
            event = event.topic0(uids);
        }

        RevokedEvent::new(event)
    }
}

pub struct RevokedEvent<M: Middleware + 'static> {
    inner: Event<Arc<M>, M, RevokedFilter>,
}

impl<M: Middleware> RevokedEvent<M> {
    pub fn new(filter: Event<Arc<M>, M, RevokedFilter>) -> Self {
        Self { inner: filter }
    }

    pub async fn stream(&self) -> Result<BoxStream<Result<Revoked, Error<M>>>, Error<M>> {
        let stream = self
            .inner
            .stream()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Revoked::from(e?)));
        Ok(stream.boxed())
    }

    pub async fn query(&self) -> Result<Vec<Revoked>, Error<M>> {
        let logs = self
            .inner
            .query()
            .await
            .map_err(Error::ContractError)?
            .iter()
            .map(|e| Revoked::from(e.clone()))
            .collect();

        Ok(logs)
    }
}

impl<M: Middleware> RevokedEvent<M>
where
    M::Provider: PubsubClient,
{
    pub async fn subscribe(
        &self,
    ) -> Result<impl Stream<Item = Result<Revoked, Error<M>>> + '_, Error<M>> {
        let stream = self
            .inner
            .subscribe()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Revoked::from(e?)));
        Ok(stream)
    }
}
