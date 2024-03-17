use std::sync::Arc;

use ethers::abi::{encode, Token};
use ethers::contract::{EthLogDecode, Event};
use ethers::providers::{Middleware, PendingTransaction, PubsubClient};

use crate::contracts::eas::{
    Attestation, AttestationRequest, AttestedFilter, MultiAttestationRequest,
};
use ethers::types::{BlockNumber, H256};

use crate::contracts::AttestationData;

use ::ethers::core::types::{Address, Bytes, U256};
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};

use tracing::debug;

use crate::eas::{Error, EAS};
use crate::transaction::{Error as TxError, PendingTx};

pub struct AttestationDataBuilder {
    pub recipient: Option<Address>,
    pub expiration_time: Option<u64>,
    pub revocable: bool,
    pub ref_uid: Option<[u8; 32]>,
    pub data: Option<Bytes>,
    pub value: Option<U256>,
}

#[derive(Debug)]
pub struct Attested {
    pub recipient: Address,
    pub attester: Address,
    pub uid: H256,
    pub schema: H256,
}

pub struct MultiAttestRequest {
    pub schema: H256,
    pub data: Vec<AttestationData>,
}

impl From<AttestedFilter> for Attested {
    fn from(event: AttestedFilter) -> Self {
        Self {
            recipient: event.recipient,
            attester: event.attester,
            uid: H256::from_slice(&event.uid),
            schema: H256::from_slice(&event.schema),
        }
    }
}

impl Default for AttestationDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationDataBuilder {
    pub fn new() -> Self {
        Self {
            recipient: None,
            expiration_time: None,
            revocable: false,
            ref_uid: None,
            data: None,
            value: None,
        }
    }

    pub fn recipient(mut self, recipient: Address) -> Self {
        self.recipient = Some(recipient);
        self
    }

    pub fn expiration_time(mut self, expiration_time: u64) -> Self {
        self.expiration_time = Some(expiration_time);
        self
    }

    pub fn revocable(mut self, revocable: bool) -> Self {
        self.revocable = revocable;
        self
    }

    pub fn ref_uid(mut self, ref_uid: [u8; 32]) -> Self {
        self.ref_uid = Some(ref_uid);
        self
    }

    pub fn data(mut self, data: &[Token]) -> Self {
        self.data = Some(encode(data).into());
        self
    }

    pub fn value(mut self, value: U256) -> Self {
        self.value = Some(value);
        self
    }

    pub fn build(self) -> AttestationData {
        AttestationData {
            recipient: self.recipient.unwrap_or(Address::zero()),
            expiration_time: self.expiration_time.unwrap_or(0),
            revocable: self.revocable,
            ref_uid: self.ref_uid.unwrap_or([0; 32]),
            data: self.data.unwrap_or_default(),
            value: self.value.unwrap_or(U256::zero()),
        }
    }
}

impl<M: Middleware + 'static> EAS<M> {
    pub async fn attest(
        &self,
        schema: &H256,
        data: AttestationData,
    ) -> Result<PendingTx<Attested>, Error<M>> {
        let attestation_request = AttestationRequest {
            schema: schema.to_fixed_bytes(),
            data,
        };

        let binding = self.contract.attest(attestation_request);
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

            let attested = AttestedFilter::decode_log(&log.clone().into())?;

            Ok(attested.into())
        };

        Ok(PendingTx::new(future))
    }

    pub async fn get_attestation(&self, attestation_id: &H256) -> Result<Attestation, Error<M>> {
        let binding = self
            .contract
            .get_attestation(attestation_id.to_fixed_bytes());

        let attestation_data = binding.call().await?;
        Ok(attestation_data)
    }

    pub async fn is_attestation_valid(&self, attestation_id: &H256) -> Result<bool, Error<M>> {
        let binding = self
            .contract
            .is_attestation_valid(attestation_id.to_fixed_bytes());

        let is_valid = binding.call().await?;
        Ok(is_valid)
    }

    pub async fn multi_attest(
        &self,
        request: Vec<MultiAttestRequest>,
    ) -> Result<PendingTx<Vec<Result<Attested, TxError>>>, Error<M>> {
        let attestation_requests = request
            .iter()
            .map(|req| MultiAttestationRequest {
                schema: req.schema.to_fixed_bytes(),
                data: req.data.clone(),
            })
            .collect();

        let binding = self.contract.multi_attest(attestation_requests);
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

            let attested: Vec<Result<Attested, TxError>> = receipt
                .logs
                .iter()
                .map(|log| {
                    AttestedFilter::decode_log(&log.clone().into())
                        .map(Into::into)
                        .map_err(TxError::from)
                })
                .collect();

            Ok(attested)
        };

        Ok(PendingTx::new(future))
    }

    pub fn attested_event<T>(
        &self,
        start_block: T,
        end_block: Option<T>,
        uids: Option<Vec<H256>>,
    ) -> AttestedEvent<M>
    where
        T: Into<BlockNumber>,
    {
        let mut event = self.contract.attested_filter().from_block(start_block);

        if let Some(end_block) = end_block {
            event = event.to_block(end_block);
        }

        if let Some(uids) = uids {
            event = event.topic0(uids);
        }

        AttestedEvent::new(event)
    }
}

pub struct AttestedEvent<M: Middleware + 'static> {
    inner: Event<Arc<M>, M, AttestedFilter>,
}

impl<M: Middleware> AttestedEvent<M> {
    pub fn new(filter: Event<Arc<M>, M, AttestedFilter>) -> Self {
        Self { inner: filter }
    }

    pub async fn stream(&self) -> Result<BoxStream<Result<Attested, Error<M>>>, Error<M>> {
        let stream = self
            .inner
            .stream()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Attested::from(e?)));
        Ok(stream.boxed())
    }

    pub async fn query(&self) -> Result<Vec<Attested>, Error<M>> {
        let logs = self
            .inner
            .query()
            .await
            .map_err(Error::ContractError)?
            .iter()
            .map(|e| Attested::from(e.clone()))
            .collect();

        Ok(logs)
    }
}

impl<M: Middleware> AttestedEvent<M>
where
    M::Provider: PubsubClient,
{
    pub async fn subscribe(
        &self,
    ) -> Result<impl Stream<Item = Result<Attested, Error<M>>> + '_, Error<M>> {
        let stream = self
            .inner
            .subscribe()
            .await
            .map_err(Error::ContractError)?
            .map(|e| Ok(Attested::from(e?)));
        Ok(stream)
    }
}
