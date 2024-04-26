use borsh::{BorshDeserialize, BorshSerialize};
use l1x_sdk::{
    caller_address, contract, contract_owner_address, emit_event_experimental,
    store::{LookupMap, Vector},
    types::{Address, U128},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy)]
struct OwnerInfo {
    address: Address,
    token_idx: u32,
}

impl OwnerInfo {
    pub fn new(address: Address, token_idx: u32) -> Self {
        Self { address, token_idx }
    }
}

/// Key for the storage of the contract data.
const STORAGE_CONTRACT_KEY: &[u8] = b"state";

/// Key for the storage of the balance data.
const STORAGE_BALANCE_OF_KEY: &[u8] = b"balances";

/// Key for the storage of token ids owned by a user
const STORAGE_BALANCE_IDS_KEY: &[u8] = b"ids";

/// Key for the storage of the ownership data.
const STORAGE_OWNER_OF_KEY: &[u8] = b"owners";

/// Key for the storage of approved data.
const STORAGE_GET_APPROVED_KEY: &[u8] = b"approved";

/// Key for the storage of the approval status data.
const STORAGE_IS_APPROVED_FOR_ALL_KEY: &[u8] = b"approved-all";

/// Token Total Supply Configuration
const L1X_NFT_TOTAL_SUPPLY: u128 = 10_000u128;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct NFTMetadata {
    name: String,
    decimals: u8,
    symbol: String,
    icon: Option<String>,
    uri: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum NftEvent {
    NftTokenMinted(String),
    NftTokenBurned(String),
    NftTokenApproved(String),
    NftTokenApprovedForAll(String),
    NftTokenTransfered(String),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct NftContract {
    metadata: NFTMetadata,
    current_token_id: u128,
    minted_total: u128,
    balance_of: LookupMap<Address, Vector<u128>>,
    owner_of: LookupMap<u128, OwnerInfo>,
    get_approved: LookupMap<u128, Address>,
    is_approved_for_all: LookupMap<Address, BTreeMap<Address, bool>>,
    burned_nfts: BTreeSet<u128>,
}

#[contract]
impl NftContract {
    pub fn new(metadata: NFTMetadata) {
        assert_eq!(
            caller_address(),
            contract_owner_address(),
            "Only the contract owner can call this method"
        );
        assert!(
            l1x_sdk::storage_read(STORAGE_CONTRACT_KEY).is_none(),
            "The contract is already initialized"
        );

        let mut contract = Self {
            metadata,
            current_token_id: 0u128,
            minted_total: 0u128,
            balance_of: LookupMap::new(STORAGE_BALANCE_OF_KEY.to_vec()),
            owner_of: LookupMap::new(STORAGE_OWNER_OF_KEY.to_vec()),
            get_approved: LookupMap::new(STORAGE_GET_APPROVED_KEY.to_vec()),
            is_approved_for_all: LookupMap::new(STORAGE_IS_APPROVED_FOR_ALL_KEY.to_vec()),
            burned_nfts: BTreeSet::new(),
        };
        contract.save();
    }

    pub fn nft_name() -> String {
        let contract = Self::load();
        contract.metadata.name
    }

    pub fn nft_symbol() -> String {
        let contract = Self::load();
        contract.metadata.symbol
    }

    pub fn nft_decimals() -> u8 {
        let contract = Self::load();
        contract.metadata.decimals
    }

    pub fn nft_icon() -> Option<String> {
        let contract = Self::load();
        contract.metadata.icon
    }

    pub fn nft_token_uri(id: U128) -> String {
        let contract = Self::load();
        contract.metadata.uri + &id.0.to_string() + ".json"
    }

    pub fn nft_metadata() -> NFTMetadata {
        let contract = Self::load();
        contract.metadata
    }

    pub fn nft_minted_total() -> U128 {
        let contract = Self::load();
        contract.minted_total.into()
    }

    pub fn nft_mint_to(to: Address) -> U128 {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        let new_token_id = contract.mint_to(to);

        // Save the contract state
        contract.save();

        new_token_id.into()
    }

    pub fn nft_mint_id_to(to: Address, id: U128) -> U128 {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        let new_token_id = contract.mint_id_to(to, id.into());

        // Save the contract state
        contract.save();

        new_token_id.into()
    }

    pub fn nft_burn(id: U128) {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        contract.burn(id.into());

        // Save the contract state
        contract.save();
    }

    pub fn nft_approve(spender: Address, id: U128) {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        contract.approve(spender, id.into());

        // Save the contract state
        contract.save();
    }

    pub fn nft_set_approval_for_all(operator: Address, approved: bool) {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        contract.set_approval_for_all(operator, approved);

        // Save the contract state
        contract.save();
    }

    pub fn nft_transfer_from(from: Address, to: Address, id: U128) {
        // load the contract storage state
        let mut contract = Self::load();

        // Call the internal implementation
        contract.transfer_from(from, to, id.into());

        // Save the contract state
        contract.save();
    }

    pub fn nft_balance_of(owner: Address) -> U128 {
        // load the contract storage state
        let contract = Self::load();

        // Call the internal implementation
        contract.balance_of(owner).into()
    }

    pub fn nft_owner_of(id: U128) -> Address {
        // load the contract storage state
        let contract = Self::load();

        // Call the internal implementation
        contract.owner_of(id.into())
    }

    pub fn nft_owned_tokens(owner: Address) -> Vec<U128> {
        // load the contract storage state
        let contract = Self::load();

        // Call the internal implementation
        contract.owned_tokens(owner)
    }
}

impl NftContract {
    fn internal_new_balance_vec(&self, address: &Address) -> Vector<u128> {
        Vector::<u128>::new([&address.to_vec(), STORAGE_BALANCE_IDS_KEY].concat())
    }

    fn internal_remove_token(&mut self, id: u128) -> (Address, u32) {
        // Update the balances
        let owner_info = self
            .owner_of
            .get(&id)
            .cloned()
            .unwrap_or_else(|| panic!("No owner with id {}", id));

        let balance_from = self
            .balance_of
            .get_mut(&owner_info.address)
            .expect("Not enough funds to burn or transfer");

        let is_last = owner_info.token_idx == balance_from.len() - 1;
        // 1. Removes the token from the Vector. The removed token is replaced by the last element in the Vector
        balance_from.swap_remove(owner_info.token_idx);
        // 2. If it is the last Vector element, additional work is not required
        if !is_last {
            // 3. Update the reference in owner_of because the token index in Vector has been changed
            let swapped_token_id = balance_from
                .get(owner_info.token_idx)
                .expect("Can't get the swapped token_id");
            let owner_ref = self
                .owner_of
                .get_mut(swapped_token_id)
                .expect("Can't find an owner of the swapped token_id");
            owner_ref.token_idx = owner_info.token_idx;
        }

        //  delete the token_id entry from `owner` and `get_approved` state
        self.owner_of.remove(id);
        self.get_approved.remove(id);

        (owner_info.address, balance_from.len())
    }

    fn internal_add_token_to(&mut self, to: Address, id: u128) {
        // Update the balances
        let balance_to = if let Some(v) = self.balance_of.get_mut(&to) {
            v
        } else {
            let new_vec = self.internal_new_balance_vec(&to);
            self.balance_of.insert(to, new_vec);
            self.balance_of
                .get_mut(&to)
                .expect("Can't get the just added Vector")
        };

        balance_to.push(id);

        let last_idx = balance_to.len() - 1;
        self.owner_of.insert(id, OwnerInfo::new(to, last_idx));
    }

    fn mint_id_to(&mut self, to: Address, id: u128) -> u128 {
        let new_token_id = id;
        assert!(
            !self.burned_nfts.contains(&id),
            "Burned Token ID {:?} cannot be minted again",
            new_token_id
        );
        assert!(
            !self.owner_of.contains_key(&new_token_id),
            "Token ID {:?} already exist",
            new_token_id
        );
        assert!(new_token_id <= L1X_NFT_TOTAL_SUPPLY, "Max supply reached");

        self.internal_add_token_to(to, new_token_id);

        self.minted_total += 1;

        // Emit the Token minted event
        emit_event_experimental(NftEvent::NftTokenMinted(format!(
            "Minted token {:#?} for owner {}",
            new_token_id, to
        )));

        l1x_sdk::msg(&format!(
            "Minted token {:#?} for owner {}",
            new_token_id, to
        ));

        new_token_id
    }

    fn mint_to(&mut self, to: Address) -> u128 {
        let mut new_token_id: u128 = self.current_token_id + 1;

        // Find the closed available id
        while new_token_id <= L1X_NFT_TOTAL_SUPPLY {
            if !self.owner_of.contains_key(&new_token_id) {
                break;
            }
            new_token_id += 1;
        }

        assert!(new_token_id <= L1X_NFT_TOTAL_SUPPLY, "Max supply reached");

        self.current_token_id = new_token_id;

        self.mint_id_to(to, new_token_id);

        new_token_id
    }

    fn burn(&mut self, id: u128) {
        assert_eq!(
            caller_address(),
            contract_owner_address(),
            "Only the contract owner can call this method"
        );

        assert!(
            self.owner_of.get(&id).is_some(),
            "Token ID {:#?} Not Minted or Doesn't exist",
            id
        );

        let (from, balance_from) = self.internal_remove_token(id);

        // update id to burned_nfts storage
        self.burned_nfts.insert(id);
        // Emit the Token burned event
        emit_event_experimental(NftEvent::NftTokenBurned(format!(
            "Burn Token_ID {:#?} from Owner {} Balance {:#?}",
            id, from, balance_from
        )));

        l1x_sdk::msg(&format!(
            "Burn Token_ID {:#?} from Owner {} Balance {:#?}",
            id, from, balance_from
        ));
    }

    fn approve(&mut self, spender: Address, id: u128) {
        // Get the caller Address
        let caller_id = l1x_sdk::caller_address();

        // Check if the ID exists in the contract's owner_of mapping or assign default
        assert!(
            self.owner_of.get(&id).is_some(),
            "TokenId: {:#?} is not minted or doesn't exist in the contract",
            &id,
        );

        let owner = self.owner_of.get(&id).cloned().unwrap();

        let caller_is_owner = caller_id == owner.address;
        let is_approved_operator = self
            .is_approved_for_all
            .get(&owner.address)
            .and_then(|approved_map| approved_map.get(&caller_id))
            .copied()
            .unwrap_or(false);

        assert!(
            caller_is_owner || is_approved_operator,
            "Caller {} is not Owner and is not an authorized operator by the Owner: {}",
            &caller_id,
            &owner.address
        );

        // Authorize the spender for the given ID
        self.get_approved.insert(id, spender.clone());

        // Emit the approval done event
        emit_event_experimental(NftEvent::NftTokenApproved(format!(
            "Approval done for token_id {:#?} from Owner {} for Spender {}",
            id, owner.address, spender
        )));

        l1x_sdk::msg(&format!(
            "Approval done for token_id {:#?} from Owner {} for Spender {}",
            id, owner.address, spender
        ));
    }

    fn set_approval_for_all(&mut self, operator: Address, approved: bool) {
        // Get the caller Address
        let caller_id = l1x_sdk::caller_address();

        // Modify the state of `is_approved_for_all`
        if let Some(approved_map) = self.is_approved_for_all.get_mut(&caller_id) {
            // Borrow the value as mutable using `get_mut` and then insert the new key-value pair
            approved_map.insert(operator.clone(), approved);
        } else {
            // If the entry doesn't exist, create a new map, insert the pair, and then insert the new map into `is_approved_for_all`
            let mut new_approved_map = BTreeMap::new();
            new_approved_map.insert(operator.clone(), approved);
            self.is_approved_for_all
                .insert(caller_id.clone(), new_approved_map);
        }

        // Emit the approval for All done event
        emit_event_experimental(NftEvent::NftTokenApprovedForAll(format!(
            "Approval-For-All done from Caller {} Operator {} Approved {:#?}",
            caller_id, operator, approved
        )));

        l1x_sdk::msg(&format!(
            "Approval-For-All done from Caller {} Operator {} Approved {:#?}",
            caller_id, operator, approved
        ));
    }

    fn transfer_from(&mut self, from: Address, to: Address, id: u128) {
        let caller_id = l1x_sdk::caller_address();

        // Check if the ID exists in the contract's owner_of mapping or assign default
        assert!(
            self.owner_of.get(&id).is_some(),
            "TokenId: {:#?} is not minted or doesn't exist in the contract",
            &id,
        );

        let owner_info = self
            .owner_of
            .get(&id)
            .cloned()
            .unwrap_or_else(|| panic!("No owner with id {}", id));

        assert_eq!(
            from, owner_info.address,
            "Not Authorized, From: {} & Owner: {} mismatch",
            from, &owner_info.address
        );

        let caller_is_owner = owner_info.address == caller_id;
        let is_approved_operator = {
            self.is_approved_for_all
                .get(&from)
                .and_then(|approved_map| approved_map.get(&caller_id))
                .copied()
                .unwrap_or(false)
        };
        let is_approved_spender = {
            let spender_id = self.get_approved.get(&id);
            spender_id == Some(&caller_id)
        };

        assert!(caller_is_owner || is_approved_operator || is_approved_spender,
            "Not Authorized, the caller, neither an owner, nor an approved spender, nor an approved operator,
             CallerId: {}, Token Owner: {}, From: {}, TokenID: {}", caller_id, owner_info.address, from, id);

        self.internal_remove_token(id);
        self.internal_add_token_to(to, id);

        // Emit transfer done event
        emit_event_experimental(NftEvent::NftTokenTransfered(format!(
            "Token Transfer done for Token_id {:#?} From {} To {}",
            id, from, to
        )));

        l1x_sdk::msg(&format!(
            "Token Transfer done for Token_id {:#?} From {} To {}",
            id, from, to
        ));
    }

    fn balance_of(&self, owner: Address) -> u128 {
        if let Some(balance) = self.balance_of.get(&owner) {
            balance.len().into()
        } else {
            0
        }
    }

    fn owner_of(&self, id: u128) -> Address {
        let owner = self.owner_of.get(&id);

        // Check if the ID exists in the contract's owner_of mapping or assign default
        assert!(
            owner.is_some(),
            "TokenId: {:#?} is not minted or doesn't exist in the contract",
            &id,
        );

        owner.unwrap().address.clone()
    }

    fn owned_tokens(&self, owner: Address) -> Vec<U128> {
        let issued_tokens = self
            .balance_of
            .get(&owner)
            .unwrap_or_else(|| panic!("Not enough funds"));

        let mut result = Vec::with_capacity(issued_tokens.len() as usize);
        for idx in 0..issued_tokens.len() {
            result.push(issued_tokens.get(idx).copied().unwrap().into())
        }

        result
    }

    fn load() -> Self {
        match l1x_sdk::storage_read(STORAGE_CONTRACT_KEY) {
            Some(bytes) => Self::try_from_slice(&bytes).unwrap(),
            None => panic!("The contract isn't initialized"),
        }
    }

    fn save(&mut self) {
        l1x_sdk::storage_write(STORAGE_CONTRACT_KEY, &self.try_to_vec().unwrap());
    }
}
