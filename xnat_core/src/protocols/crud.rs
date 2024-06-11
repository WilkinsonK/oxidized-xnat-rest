use std::{future::Future, iter::IntoIterator, pin::Pin, vec::IntoIter};

use async_trait::async_trait;
use reqwest::{Response, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Complex type representing a pinned future with
/// a specified output.
type PinnedFuture<'f, O> = Pin<Box<dyn Future<Output = anyhow::Result<O>> + 'f>>;

/// Errors specific to the purpose of interactions
/// between an XNAT client and the host during
/// protocol transactions.
#[derive(Debug, Error)]
pub enum CrudError {
    #[error("host XNAT experienced an internal error ({0})")]
    HostError(StatusCode),
    #[error("resource is not available ({0})")]
    NotAvailable(StatusCode),
    #[error("could not create resource ({0})")]
    NotCreated(StatusCode),
    #[error("could not retrieve resource ({0})")]
    NotFound(StatusCode),
}

/// Type is able to implement CREATE requests for
/// a particular model. Upon creation, these
/// methods are expected to then return the
/// individual results, and then a
/// `Ok(Self::Model)` response if the request is
/// successful.
#[async_trait(?Send)]
pub trait Create {
    type Model: Clone + Serialize;

    /// Attempt to send a CREATE request to the
    /// XNAT server for **multiple** models.
    #[inline(never)]
    fn create_many<M>(&self, models: M) -> Vec<PinnedFuture<'_, Self::Model>>
    where
        M: IntoIterator<Item = Self::Model, IntoIter = IntoIter<Self::Model>>,
    {
        models
            .into_iter()
            .map(|m| self.create_once(m))
            .collect::<Vec<_>>()
    }
    /// Attempt to send a CREATE request to the
    /// XNAT server for **one** model.
    async fn create_once(&self, model: Self::Model) -> anyhow::Result<Self::Model>;
}

/// Type is able to implement RETRIEVE requests
/// for a particular model.
#[async_trait(?Send)]
pub trait Retrieve<M>
where
    M: Clone + DeserializeOwned,
{
    /// Get all instances of a particular model
    /// available to the user via the XNAT host.
    #[inline(never)]
    async fn get_all(&self) -> anyhow::Result<Vec<M>>
    where
        M: Default,
    {
        self.get_any_from(&M::default()).await
    }
    /// Get all instances of a particular model
    /// using another model as the query
    /// parameters for the request.
    async fn get_any_from(&self, model: &M) -> anyhow::Result<Vec<M>>;
    /// Get one instance of a particular model
    /// using another model as the query
    /// parameters for the request.
    #[inline(never)]
    async fn get_one_from(&self, model: &M) -> anyhow::Result<M>
    {
        match self.get_any_from(model).await?.first() {
            None => Err(CrudError::NotFound(StatusCode::from_u16(400)?).into()),
            Some(m) => Ok(m.to_owned())
        }
    }
}

/// Type is able to implement UPDATE or UPSERT
/// requests for a particular model.
#[async_trait(?Send)]
pub trait Update {
    type Model: Clone + DeserializeOwned;

    /// Attempt to send **multiple** UPDATE
    /// requests to the XNAT host.
    #[inline(never)]
    fn update_many<M>(&self, models: M) -> Vec<PinnedFuture<'_, Self::Model>>
    where
        M: IntoIterator<Item = Self::Model, IntoIter = IntoIter<Self::Model>>,
    {
        models
            .into_iter()
            .map(|m| self.update_once(m))
            .collect::<Vec<_>>()
    }
    /// Attempt to send **one** UPDATE request to
    /// the XNAT host.
    async fn update_once(&self, model: Self::Model) -> anyhow::Result<Self::Model>;
}

/// Type is able to implement DELETE requests for
/// a particular model.
#[async_trait(?Send)]
pub trait Delete {
    type Model: DeserializeOwned;

    /// Attempt to send **multiple** DELETE
    /// requests to the XNAT host.
    #[inline(never)]
    fn delete_many<M>(&self, models: M) -> Vec<PinnedFuture<'_, Self::Model>>
    where
        M: IntoIterator<Item = Self::Model, IntoIter = IntoIter<Self::Model>>,
    {
        models
            .into_iter()
            .map(|m| self.delete_once(m))
            .collect::<Vec<_>>()
    }
    /// Attempt to send **one** DELETE request to
    /// the XNAT host.
    async fn delete_once(&self, model: Self::Model) -> anyhow::Result<Self::Model>;
}

/// Utility function to shortcut the handling of
/// the returning value of a sent request.
pub async fn try_retrieve<T, Callback, F>(response: Response, call: Callback) -> anyhow::Result<T>
where
    F: Future<Output = T>,
    Callback: FnOnce(Response) -> F,
{
    let status = response.status();

    if status.is_success() {
        Ok(call(response).await)
    } else if status.is_client_error() && status == 400 {
        Err(CrudError::NotFound(status).into())
    } else if status.is_client_error() {
        Err(CrudError::NotAvailable(status).into())
    } else {
        Err(CrudError::HostError(status).into())
    }
}