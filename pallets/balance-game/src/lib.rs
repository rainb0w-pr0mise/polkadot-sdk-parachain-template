//! # Balance Game Pallet
//!
//! A Substrate pallet implementing an interactive balance-based game. Users can accumulate, set, and clear temporary balances,
//! and update their actual balances. The pallet includes a win condition based on a "magic number," where users can win if their
//! balance or the total balance across all users reaches the magic number. Operations are tracked to ensure limits are not exceeded.
//!
//! **This pallet is a demonstration of FRAME concepts and is not intended for production use.**
//!
//! > Built with *Substrate* for *Polkadot*.
//!
//! ### Key Features
//! - **Temporary Balances**: Users can accumulate and clear temporary balances.
//! - **Balance Updates**: Temporary balances can be transferred to actual balances.
//! - **Win Condition**: Users win if their balance or the total balance reaches a predefined "magic number."
//! - **Operation Tracking**: Limits the number of operations per block to prevent abuse.
//! - **Leaderboard**: Tracks users who have won the game.
//!
//! ### Pallet Sections
//! - **Configuration**: Defines types and parameters like `MagicNumber` and `OperationMax`.
//! - **Storage**: Manages temporary balances, actual balances, total balances, and operation counts.
//! - **Events**: Emits events for balance updates, wins, and operation counts.
//! - **Errors**: Handles errors like insufficient balance, operation limits, and missing entries.
//! - **Dispatchable Functions**: Provides extrinsics for interacting with the pallet.
//! - **Helper Functions**: Implements logic for accumulating balances, tracking operations, and checking win conditions.
//!
//! ### Usage
//! This pallet is designed to help developers understand FRAME pallet development. For more information, refer to:
//! - [Your First Pallet Tutorial](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html)
//! - [FRAME Documentation](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html)
//! - [Kitchen Sink Pallet](https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html)
//!
//! Run `cargo doc --package pallet-template --open` to view detailed documentation.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::Blake2_128Concat;
use frame_support::{dispatch::DispatchResult, traits::Get, weights::Weight};
use frame_system::ensure_signed;
use sp_runtime::traits::{Saturating, StaticLookup};

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//
// To see a full list of `pallet` macros and their use cases, see:
// <https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/index.html>

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;

    #[pallet::config]
    pub trait Config: pallet_balances::Config + frame_system::Config {
        #[pallet::constant]
        type MagicNumber: Get<Self::Balance>;

        #[pallet::constant]
        type OperationMax: Get<u16>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Resets temporary balances and clears storage at the start of each block.
        ///
        /// # Returns
        /// - The weight consumed by the operation.
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Reset temporary balances at the start of each block
            for (key, _) in TemporaryBalance::<T>::iter() {
                TemporaryBalance::<T>::remove(key.clone());
                UserBalances::<T>::remove(key);
            }
            TotalBalance::<T>::kill();

            // Return the weight consumed by `on_initialize`
            Weight::zero()
        }

        /// Performs cleanup and checks the win condition at the end of each block.
        fn on_finalize(_n: BlockNumberFor<T>) {
            // Perform necessary data/state clean up here.
            Self::check_win_condition();
        }
    }

    #[pallet::call(weight(<T as Config>::WeightInfo))]
    impl<T: Config> Pallet<T> {
        /// Allows a user to increase their temporary balance by a specified amount.
        ///
        /// # Arguments
        /// - `who`: The origin of the call (must be a signed account).
        /// - `increase_by`: The amount by which to increase the temporary balance.
        ///
        /// # Errors
        /// - `NoEntryForSender`: If the caller does not have an existing temporary balance.
        ///
        /// # Events
        /// - `AccumulateTemporaryBalance`: Emitted when the temporary balance is successfully increased.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::accumulate_temporary_balance())]
        pub fn accumulate_temporary_balance(
            who: OriginFor<T>,
            increase_by: T::Balance,
        ) -> DispatchResult {
            Self::do_accumulate_temporary_balance(who, increase_by)
        }

        /// Clears the temporary balance for a specified user.
        ///
        /// # Arguments
        /// - `origin`: The origin of the call (must be a signed account).
        /// - `who`: The account whose temporary balance will be cleared.
        ///
        /// # Events
        /// - `TemporaryBalanceCleared`: Emitted when the temporary balance is successfully cleared.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::clear_temporary_balance())]
        pub fn clear_temporary_balance(
            origin: OriginFor<T>,
            who: AccountIdLookupOf<T>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let who = T::Lookup::lookup(who)?;
            let clear_temporary_balance_key: u32 = 2;

            TemporaryBalance::<T>::remove(&who);
            Self::record_operation(clear_temporary_balance_key)?;
            Self::deposit_event(Event::TempoaryBalanceCleared(who.clone()));
            Ok(())
        }

        /// Transfers the temporary balance to the user's actual balance.
        ///
        /// # Arguments
        /// - `origin`: The origin of the call (must be a signed account).
        ///
        /// # Errors
        /// - `TemporaryBalanceNotFound`: If the caller does not have a temporary balance.
        ///
        /// # Events
        /// - `BalanceUpdated`: Emitted when the balance is successfully updated.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::update_balance())]
        pub fn update_balance(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let update_balance_key: u32 = 3;

            // Get the new balance from TemporaryBalance storage
            let new_balance =
                TemporaryBalance::<T>::get(&who).ok_or(Error::<T>::TempoaryBalanceNotFound)?;

            // Update the user's balance
            UserBalances::<T>::try_mutate(&who, |balance| -> DispatchResult {
                // Ensure that the balance is initialized to avoid issues with None
                let current_balance = balance.unwrap_or_else(Zero::zero);
                let updated_balance = current_balance.saturating_add(new_balance);
                Self::record_operation(update_balance_key)?;
                *balance = Some(updated_balance);
                Ok(())
            })?;

            // Update the total balance in TotalBalance storage
            TotalBalance::<T>::mutate(|total| {
                *total = total.saturating_add(new_balance);
            });

            // Emit the event
            Self::deposit_event(Event::BalanceUpdated(who.clone(), new_balance));
            Ok(())
        }
    }

    /// Events are a simple means of reporting specific conditions and
    /// circumstances that have happened that users, Dapps and/or chain explorers would find
    /// interesting and otherwise difficult to detect.
    #[pallet::event]
    /// This attribute generate the function `deposit_event` to deposit one of this pallet event,
    /// it is optional, it is also possible to provide a custom implementation.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// TemporaryBalance event, just here so there's a generic type that's used.
        AccumulateTemporaryBalance {
            balance: T::Balance,
        },
        SetTemporaryBalance {
            balance: T::Balance,
        },
        TempoaryBalanceCleared(T::AccountId),
        BalanceUpdated(T::AccountId, T::Balance),
        GameWon(T::AccountId, T::Balance),
        OperationCountUpdated(u32, u16), // TODO: Max Operation reached, wait till next block, with the Magic Nuimber.
    }

    #[pallet::error]
    pub enum Error<T> {
        NoEntryForSender,
        ValueAlreadySet,
        InsufficientBalance,
        ExceedsWithdrawalLimit,
        NoHoldFound,
        InsufficientAmount,
        TempoaryBalanceNotFound,
        OperationLimitExceeded,
    }

    // pallet::storage attributes allow for type-safe usage of the Substrate storage database,
    // so you can keep things around between blocks.
    //
    // Any storage must be one of `StorageValue`, `StorageMap`, or `StorageDoubleMap`.
    // The first generic holds the prefix to use and is generated by the macro.
    // The query kind is either `OptionQuery` (the default) or `ValueQuery`.
    // Below are examples with the correct methods for each type of storage:

    // Example for `StorageMap` using `Twox64Concat` hasher:
    // `type TemporaryBalance<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance, OptionQuery>`;
    // Methods:
    // - `TemporaryBalance::insert(who: T::AccountId, new_value: T::Balance);`  // Inserts a new value.
    // - `TemporaryBalance::remove(who: &T::AccountId);`                        // Removes the value associated
    //   with the key.
    // - `TemporaryBalance::contains_key(who: &T::AccountId) -> bool;`          // Checks if a key exists.
    // - `TemporaryBalance::get(who: &T::AccountId) -> Option<T::Balance>;`     // Retrieves the value
    //   associated with a key.

    // Example for `StorageValue` with `ValueQuery`:
    // `type TotalBalance<T: Config> = StorageValue<_, T::Balance, ValueQuery>`;
    // Methods:
    // - `TotalBalance::get() -> T::Balance;`              // Retrieves the stored value.
    // - `TotalBalance::put(new_value: T::Balance);`       // Stores a new value.
    // - `TotalBalance::mutate(|v| *v += 1);`              // Mutates the stored value.
    // - `TotalBalance::kill();`                           // Removes the stored value, sets it to default.

    // Example for `CountedStorageMap`:
    // `type OperationCounts<T: Config> = CountedStorageMap<_, Blake2_128Concat, u8, u16>`;
    // Methods:
    // - `OperationCounts::insert(key: u8, value: u16);`       // Inserts a key-value pair.
    // - `OperationCounts::remove(key: &u8);`                 // Removes a key-value pair.
    // - `OperationCounts::get(key: &u8) -> Option<u16>;`     // Retrieves the value for a key.
    // - `OperationCounts::iter() -> impl Iterator<Item=(u8, u16)>;` // Iterates over all key-value
    //   pairs.
    // - `OperationCounts::count() -> u32;`                   // Returns the count of stored items.
    #[pallet::storage]
    pub(super) type UserBalances<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance>;
    /// Holds tempoary balance.
    #[pallet::storage]
    pub(super) type TemporaryBalance<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, T::Balance, OptionQuery>;
    /// Account for an operation to take place(Intents).
    #[pallet::storage]
    pub type OperationCounts<T> = CountedStorageMap<_, Blake2_128Concat, u32, u16>;
    /// Store the total value of Balances held in storage, this one uses the query kind:
    /// `ValueQuery`, we'll demonstrate the usage of 'mutate' API.
    #[pallet::storage]
    pub(super) type TotalBalance<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    #[pallet::storage]
    pub(super) type Leaderboard<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32>;

    // The genesis config type.
    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub temporary_balance: Vec<(T::AccountId, T::Balance)>,
    }

    /// Initializes the pallet's state during blockchain genesis.
    ///
    /// # Arguments
    /// - `self`: The genesis configuration containing initial temporary balances.
    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for (a, b) in &self.temporary_balance {
                <TemporaryBalance<T>>::insert(a, b);
            }
        }
    }
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
    // Add public immutables and private mutables.
    #[allow(dead_code)]
    fn do_accumulate_temporary_balance(
        origin: frame_system::pallet_prelude::OriginFor<T>,
        increase_by: T::Balance,
    ) -> DispatchResult {
        // Ensure that the origin is signed, meaning the sender is an account.
        let who = ensure_signed(origin)?;
        let accumulate_key: u32 = 0;

        Self::record_operation(accumulate_key)?;

        // If an entry exists, we can mutate it.
        <TemporaryBalance<T>>::mutate(&who, |temp_bal_opt| {
            if let Some(temporary_balance) = temp_bal_opt {
                // Using `saturating_add` to avoid overflow and safely update the value.
                *temporary_balance = temporary_balance.saturating_add(increase_by);
                Self::deposit_event(Event::AccumulateTemporaryBalance {
                    balance: increase_by,
                });
            } else {
                // If it's None, initialize it to the increase value.
                *temp_bal_opt = Some(increase_by);
            }
        });
        Ok(())
    }

    /// Records the number of operations performed for a specific key and ensures the operation limit is not exceeded.
    ///
    /// # Arguments
    /// - `key`: The operation key to track.
    ///
    /// # Errors
    /// - `OperationLimitExceeded`: If the operation limit is exceeded.
    ///
    /// # Events
    /// - `OperationCountUpdated`: Emitted when the operation count is updated.
    fn record_operation(key: u32) -> DispatchResult {
        // Fetch the current count from the storage.
        let current_count = OperationCounts::<T>::get(key).unwrap_or_default();

        // // Check if the current count exceeds the magic number..
        frame_support::ensure!(
            current_count <= T::OperationMax::get(),
            Error::<T>::OperationLimitExceeded
        );

        // Increment the count if the limit has not been reached.
        let new_count = current_count + 1;
        OperationCounts::<T>::insert(key, new_count);

        // Emit an event indicating that the operation count has been updated.
        Self::deposit_event(Event::OperationCountUpdated(key, new_count));

        Ok(())
    }

    /// Checks if any player has reached the magic number in their balance or if the total balance
    /// reaches the magic number. If so, updates the leaderboard and emits a `GameWon` event.
    ///
    /// # Events
    /// - `GameWon`: Emitted when a player wins the game.
    pub fn check_win_condition() {
        let total_balance = TotalBalance::<T>::get();

        // Check if any player's balance reaches the magic number and emit event
        for (account, balance) in UserBalances::<T>::iter() {
            if balance == <T as Config>::MagicNumber::get() {
                Leaderboard::<T>::mutate(&account, |points_opt| {
                    if let Some(points) = points_opt {
                        // Using `saturating_add` to avoid overflow and safely update the value.
                        *points = points.saturating_add(1);
                    } else {
                        // If it's None, initialize it to the increase value.
                        *points_opt = Some(1);
                    }
                });
                Self::deposit_event(Event::GameWon(account.clone(), balance));
            }
        }

        // Emit event for all accounts if the total balance reaches the magic number
        if total_balance == <T as Config>::MagicNumber::get() {
            for (account, balance) in UserBalances::<T>::iter() {
                Leaderboard::<T>::mutate(&account, |points_opt| {
                    if let Some(points) = points_opt {
                        // Using `saturating_add` to avoid overflow and safely update the value.
                        *points = points.saturating_add(1);
                    } else {
                        // If it's None, initialize it to the increase value.
                        *points_opt = Some(1);
                    }
                });
                Self::deposit_event(Event::GameWon(account.clone(), balance));
            }
        }
    }
}
