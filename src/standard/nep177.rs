//! NEP-177 non-fungible token contract metadata implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0177.md>
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U64,
    serde::*,
    AccountId, BorshStorageKey,
};
use thiserror::Error;

use crate::{
    slot::Slot,
    standard::{
        nep171::{
            self,
            error::TokenDoesNotExistError,
            event::{NftContractMetadataUpdateLog, NftMetadataUpdateLog},
            *,
        },
        nep297::Event,
    },
    DefaultStorageKey,
};

pub use ext::*;

const CONTRACT_METADATA_NOT_INITIALIZED_ERROR: &str = "Contract metadata not initialized";

/// Non-fungible token with metadata.
#[derive(
    Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    /// The token ID.
    pub token_id: TokenId,
    /// The token owner.
    pub owner_id: AccountId,
    /// The token metadata.
    pub metadata: TokenMetadata,
}

impl Token {
    /// Loads token metadata.
    pub fn load(
        contract: &(impl Nep171Controller + Nep177Controller),
        token_id: TokenId,
    ) -> Option<Self> {
        let owner_id = contract.token_owner(&token_id)?;
        let metadata = contract.token_metadata(&token_id)?;
        Some(Self {
            token_id,
            owner_id,
            metadata,
        })
    }
}

/// Non-fungible token contract metadata.
#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractMetadata {
    /// The metadata specification version. Essentially a version like "nft-2.0.0", replacing "2.0.0" with the implemented version of NEP-177.
    pub spec: String,
    /// The name of the NFT contract, e.g. "Mochi Rising — Digital Edition" or "Metaverse 3".
    pub name: String,
    /// The symbol of the NFT contract, e.g. "MOCHI" or "M3".
    pub symbol: String,
    /// Data URI for the contract icon.
    pub icon: Option<String>,
    /// Gateway known to have reliable access to decentralized storage assets referenced by `reference` or `media` URLs.
    pub base_uri: Option<String>,
    /// URL to a JSON file with more info about the NFT contract.
    pub reference: Option<String>,
    /// Base-64-encoded SHA-256 hash of the referenced JSON file. Required if `reference` is present.
    pub reference_hash: Option<String>,
}

impl ContractMetadata {
    /// The metadata specification version.
    pub const SPEC: &'static str = "nft-2.1.0";

    /// Creates a new contract metadata, specifying the name, symbol, and
    /// optional base URI. Other fields are set to `None`.
    pub fn new(name: String, symbol: String, base_uri: Option<String>) -> Self {
        Self {
            spec: Self::SPEC.to_string(),
            name,
            symbol,
            icon: None,
            base_uri,
            reference: None,
            reference_hash: None,
        }
    }
}

/// Non-fungible token metadata.
#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {
    /// This token's title, e.g. "Arch Nemesis: Mail Carrier" or "Parcel #5055".
    pub title: Option<String>,
    /// Free-text description of this specific token.
    pub description: Option<String>,
    /// The token's image or other associated media.
    pub media: Option<String>,
    /// Base-64-encoded SHA-256 hash of the media. Required if `media` is present.
    pub media_hash: Option<String>,
    /// Number of copies of this set of metadata in existence when token was minted.
    pub copies: Option<U64>,
    /// When the token was issued, in milliseconds since the UNIX epoch.
    pub issued_at: Option<U64>,
    /// When the token expires, in milliseconds since the UNIX epoch.
    pub expires_at: Option<U64>,
    /// When the token starts being valid, in milliseconds since the UNIX epoch.
    pub starts_at: Option<U64>,
    /// When the token was last updated, in milliseconds since the UNIX epoch.
    pub updated_at: Option<U64>,
    /// Anything extra the NFT wants to store on-chain. Can be stringified JSON.
    pub extra: Option<String>,
    /// URL to an off-chain JSON file with more info about the token.
    pub reference: Option<String>,
    /// Base-64-encoded SHA-256 hash of the referenced JSON file. Required if `reference` is present.
    pub reference_hash: Option<String>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    ContractMetadata,
    TokenMetadata(&'a TokenId),
}

/// Internal functions for [`Nep177Controller`].
pub trait Nep177ControllerInternal {
    /// Storage root.
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep177)
    }

    /// Storage slot for contract metadata.
    fn slot_contract_metadata() -> Slot<ContractMetadata> {
        Self::root().field(StorageKey::ContractMetadata)
    }

    /// Storage slot for token metadata.
    fn slot_token_metadata(token_id: &TokenId) -> Slot<TokenMetadata> {
        Self::root().field(StorageKey::TokenMetadata(token_id))
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-177.
pub trait Nep177Controller {
    /// Mint a new token with metadata.
    fn mint_with_metadata(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        metadata: TokenMetadata,
    ) -> Result<(), Nep171MintError>;

    /// Burn a token with metadata.
    fn burn_with_metadata(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError>;

    /// Sets the metadata for a token ID without checking whether the token exists, etc. and emits an [`Nep171Event::NftMetadataUpdate`] event.
    fn set_token_metadata_unchecked(&mut self, token_id: TokenId, metadata: Option<TokenMetadata>);

    /// Sets the metadata for a token ID and emits an [`Nep171Event::NftMetadataUpdate`] event.
    fn set_token_metadata(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError>;

    /// Sets the contract metadata and emits an [`Nep171Event::ContractMetadataUpdate`] event.
    fn set_contract_metadata(&mut self, metadata: ContractMetadata);

    /// Returns the contract metadata.
    fn contract_metadata(&self) -> ContractMetadata;

    /// Returns the metadata for a token ID.
    fn token_metadata(&self, token_id: &TokenId) -> Option<TokenMetadata>;
}

/// Error returned when a token update fails.
#[derive(Error, Debug)]
pub enum UpdateTokenMetadataError {
    /// The token does not exist.
    #[error(transparent)]
    TokenNotFound(#[from] TokenDoesNotExistError),
}

impl<T: Nep177ControllerInternal + Nep171Controller> Nep177Controller for T {
    fn set_token_metadata(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError> {
        if self.token_owner(&token_id).is_some() {
            self.set_token_metadata_unchecked(token_id, Some(metadata));
            Ok(())
        } else {
            Err(TokenDoesNotExistError { token_id }.into())
        }
    }

    fn set_contract_metadata(&mut self, metadata: ContractMetadata) {
        Self::slot_contract_metadata().set(Some(&metadata));
        Nep171Event::ContractMetadataUpdate(vec![NftContractMetadataUpdateLog { memo: None }])
            .emit();
    }

    fn mint_with_metadata(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        metadata: TokenMetadata,
    ) -> Result<(), Nep171MintError> {
        let token_ids = [token_id];
        self.mint(&token_ids, &owner_id, None)?;
        let [token_id] = token_ids;
        self.set_token_metadata_unchecked(token_id, Some(metadata));
        Ok(())
    }

    fn burn_with_metadata(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError> {
        let token_ids = [token_id];
        self.burn(&token_ids, current_owner_id, None)?;
        let [token_id] = token_ids;
        self.set_token_metadata_unchecked(token_id, None);
        Ok(())
    }

    fn set_token_metadata_unchecked(&mut self, token_id: TokenId, metadata: Option<TokenMetadata>) {
        <Self as Nep177ControllerInternal>::slot_token_metadata(&token_id).set(metadata.as_ref());
        nep171::Nep171Event::NftMetadataUpdate(vec![NftMetadataUpdateLog {
            token_ids: vec![token_id],
            memo: None,
        }])
        .emit();
    }

    fn token_metadata(&self, token_id: &TokenId) -> Option<TokenMetadata> {
        <Self as Nep177ControllerInternal>::slot_token_metadata(token_id).read()
    }

    fn contract_metadata(&self) -> ContractMetadata {
        Self::slot_contract_metadata()
            .read()
            .unwrap_or_else(|| env::panic_str(CONTRACT_METADATA_NOT_INITIALIZED_ERROR))
    }
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use super::*;

    #[near_sdk::ext_contract(ext_nep171)]
    pub trait Nep177 {
        fn nft_metadata(&self) -> ContractMetadata;
    }
}
