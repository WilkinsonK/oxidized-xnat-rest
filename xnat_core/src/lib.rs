mod uri;
mod uri_admin;
mod version;

#[cfg(feature = "derive")]
pub use oxinat_derive::*;

pub use crate::uri::{UriBuilder, UriBuildError};
pub use crate::uri_admin::{AdminUri, AdminUriLegacy};
pub use crate::version::Version;
