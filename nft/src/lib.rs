use near_contract_standards::non_fungible_token::events::NftBurn;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::{
    assert_one_yocto, assert_self, env, near_bindgen, require, AccountId, BorshStorageKey,
    PanicOnDefault, PromiseOrValue,
};

mod params;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    state_version: u64,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
}

near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
//near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        let metadata = NFTContractMetadata {
            spec: NFT_METADATA_SPEC.into(),
            name: "eTEU Transfer".into(),
            symbol: "ETEUV1".into(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        };
        metadata.assert_valid();

        Self {
            state_version: 1,
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    /// Mints a new NFT
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: params::TokenCreateMetadata,
    ) -> Token {
        assert_self();

        let metadata = TokenMetadata {
            title: Some(token_metadata.title),
            description: Some(token_metadata.description),
            media: None,
            media_hash: None,
            copies: Some(1),
            issued_at: Some(env::block_timestamp().to_string()),
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        };

        self.tokens
            .internal_mint(token_id, receiver_id, Some(metadata))
    }

    /// Burns an existing NFT
    #[payable]
    pub fn nft_burn(&mut self, token_id: TokenId) {
        assert_one_yocto();

        let owner_id = self
            .tokens
            .owner_by_id
            .get(&token_id)
            .unwrap_or_else(|| env::panic_str("no such token"));

        require!(owner_id == env::predecessor_account_id(), "not token owner");

        if let Some(m) = &mut self.tokens.next_approval_id_by_id {
            m.remove(&token_id);
        }

        if let Some(m) = &mut self.tokens.approvals_by_id {
            m.remove(&token_id);
        }

        if let Some(m) = &mut self.tokens.tokens_per_owner {
            if let Some(mut token_ids) = m.get(&owner_id) {
                token_ids.remove(&token_id);
                if token_ids.is_empty() {
                    m.remove(&owner_id);
                } else {
                    m.insert(&owner_id, &token_ids);
                }
            }
        }

        if let Some(m) = &mut self.tokens.token_metadata_by_id {
            m.remove(&token_id);
        }

        self.tokens.owner_by_id.remove(&token_id).unwrap();

        NftBurn {
            owner_id: &owner_id,
            token_ids: &[&token_id],
            // TODO
            authorized_id: None,
            memo: None,
        }
        .emit();
    }
}
