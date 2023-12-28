use near_contract_standards::non_fungible_token::core::{
    NonFungibleTokenCore, NonFungibleTokenResolver,
};
use near_contract_standards::non_fungible_token::events::{NftBurn, NftTransfer};
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::{
    assert_self, env, near_bindgen, require, AccountId, BorshStorageKey,
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

        let now_ms = env::block_timestamp() / 1_000_000;

        let metadata = TokenMetadata {
            title: Some(token_metadata.title),
            description: Some(token_metadata.description),
            media: None,
            media_hash: None,
            copies: Some(1),
            issued_at: Some(now_ms.to_string()),
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

#[near_bindgen]
impl NonFungibleTokenCore for Contract {
    #[payable]
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        let sender_id = env::predecessor_account_id();

        let owner_id = self
            .tokens
            .owner_by_id
            .get(&token_id)
            .unwrap_or_else(|| env::panic_str("Token not found"));

        // clear approvals, if using Approval Management extension
        // this will be rolled back by a panic if sending fails
        let approved_account_ids = self
            .tokens
            .approvals_by_id
            .as_mut()
            .and_then(|by_id| by_id.remove(&token_id));

        // check if authorized
        let sender_id = if sender_id == self.tokens.owner_id {
            // Allow NFT owner to do anything
            None
        } else if sender_id != owner_id {
            // if approval extension is NOT being used, or if token has no approved accounts
            let app_acc_ids = approved_account_ids
                .as_ref()
                .unwrap_or_else(|| env::panic_str("Unauthorized"));

            // Approval extension is being used; get approval_id for sender.
            let actual_approval_id = app_acc_ids.get(&sender_id);

            // Panic if sender not approved at all
            if actual_approval_id.is_none() {
                env::panic_str("Sender not approved");
            }

            // If approval_id included, check that it matches
            require!(
                approval_id.is_none() || actual_approval_id == approval_id.as_ref(),
                format!(
                    "The actual approval_id {:?} is different from the given approval_id {:?}",
                    actual_approval_id, approval_id
                )
            );
            Some(sender_id)
        } else {
            None
        };

        require!(
            owner_id != receiver_id,
            "Current and next owner must differ"
        );

        self.tokens
            .internal_transfer_unguarded(&token_id, &owner_id, &receiver_id);

        NftTransfer {
            old_owner_id: &owner_id,
            new_owner_id: &receiver_id,
            token_ids: &[&token_id],
            authorized_id: sender_id
                .as_ref()
                .filter(|sender_id| *sender_id == &owner_id),
            memo: memo.as_deref(),
        }
        .emit();
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        _receiver_id: AccountId,
        _token_id: TokenId,
        _approval_id: Option<u64>,
        _memo: Option<String>,
        _msg: String,
    ) -> PromiseOrValue<bool> {
        env::panic_str("not implemented");
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id)
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<std::collections::HashMap<AccountId, u64>>,
    ) -> bool {
        self.tokens.nft_resolve_transfer(
            previous_owner_id,
            receiver_id,
            token_id,
            approved_account_ids,
        )
    }
}
