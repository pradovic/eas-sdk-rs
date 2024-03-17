use std::{
    pin::Pin,
    task::{Context, Poll},
};

use ethers::types::H256;

use ethers::core::abi::Error as AbiError;

use futures::{future::BoxFuture, Future};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("provider error: {0}")]
    Provider(#[from] ethers::providers::ProviderError),

    #[error("transaction dropped: {0}")]
    TransactionDropped(H256),

    #[error("contract error: {0}")]
    ParseLog(#[from] AbiError),

    #[error("missing logs")]
    MissingLogs,

    #[error("unknown error: {0}")]
    Other(String),
}

pub struct PendingTx<T> {
    inner: BoxFuture<'static, Result<T, Error>>,
}

impl<T> PendingTx<T> {
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = Result<T, Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(fut),
        }
    }
}

impl<T> Future for PendingTx<T> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.inner.as_mut().poll(cx)
    }
}
