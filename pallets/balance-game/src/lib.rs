//! # Balance Game Pallet
//!
//! A pallet demonstrating concepts, APIs and structures common to most FRAME runtimes in an interactive game.
//!
//! **This pallet serves as an example and is not meant to be used in production.**
//!
//! > Made with *Substrate*, for *Polkadot*.
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet.
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! To get started with pallet development, consider using this tutorial:
//!
//! <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//!
//! And reading the main documentation of the `frame` crate:
//!
//! <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
//!
//! And looking at the frame [`kitchen-sink`](https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html)
//! pallet, a showcase of all pallet macros.
//!
//! ### Pallet Sections
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//! - A **set of helper functions**.
//! - A simple transaction extension implementation (see:
//!   [`sp_runtime::traits::TransactionExtension`]) which increases the priority of the
//!   [`Call::set_temporary_balance`] if it's present and drops any transaction with an encoded length higher
//!   than 200 bytes.
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use frame_support::Blake2_128Concat;
use frame_support::{
    dispatch::{ClassifyDispatch, DispatchClass, DispatchResult, Pays, PaysFee, WeighData},
    traits::{Get, IsSubType},
    weights::Weight,
};
use frame_system::ensure_signed;
use log::info;
use scale_info::TypeInfo;
use sp_runtime::traits::{Bounded, SaturatedConversion, Saturating, StaticLookup};

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

const LOG_TARGET: &str = "runtime::pallet-balance-game";

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> = <T as pallet_balances::Config>::Balance;
type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
const MILLICENTS: u32 = 1_000_000_000;

// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//
// To see a full list of `pallet` macros and their use cases, see:
// <https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/index.html>

// A custom weight calculator tailored for the dispatch call `set_temporary_balance()`. This actually examines
// the arguments and makes a decision based upon them.
//
// The `WeightData<T>` trait has access to the arguments of the dispatch that it wants to assign a
// weight to. Nonetheless, the trait itself cannot make any assumptions about what the generic type
// of the arguments (`T`) is. Based on our needs, we could replace `T` with a more concrete type
// while implementing the trait. The `pallet::weight` expects whatever implements `WeighData<T>` to
// replace `T` with a tuple of the dispatch arguments. This is exactly how we will craft the
// implementation below.
//
// The rules of `WeightForSetTemporaryBalance` are as follows:
// - The final weight of each dispatch is calculated as the argument of the call multiplied by the
//   parameter given to the `WeightForSetTemporaryBalance`'s constructor.
// - assigns a dispatch class `operational` if the argument of the call is more than 1000.
//
// More information can be read at:
//   - https://docs.substrate.io/main-docs/build/tx-weights-fees/
//
// Manually configuring weight is an advanced operation and what you really need may well be
//   fulfilled by running the benchmarking toolchain. Refer to `benchmarking.rs` file.
struct WeightForSetTemporaryBalance<T: pallet_balances::Config>(BalanceOf<T>);

impl<T: pallet_balances::Config> WeighData<(&AccountIdLookupOf<T>, &BalanceOf<T>)>
    for WeightForSetTemporaryBalance<T>
{
    fn weigh_data(&self, target: (&AccountIdLookupOf<T>, &BalanceOf<T>)) -> Weight {
        let multiplier = self.0;
        // *target.1 is the amount passed into the extrinsic
        let cents = *target.1 / <BalanceOf<T>>::from(MILLICENTS);
        Weight::from_parts((cents * multiplier).saturated_into::<u64>(), 0)
    }
}

impl<T: pallet_balances::Config> ClassifyDispatch<(&AccountIdLookupOf<T>, &BalanceOf<T>)>
    for WeightForSetTemporaryBalance<T>
{
    fn classify_dispatch(&self, target: (&AccountIdLookupOf<T>, &BalanceOf<T>)) -> DispatchClass {
        // current_balance + target amount passed into the extrinsic.
        if self.0.saturating_add(*target.1) > <BalanceOf<T>>::from(1000u32) {
            DispatchClass::Operational
        } else {
            DispatchClass::Normal
        }
    }
}

impl<T: pallet_balances::Config> PaysFee<(&AccountIdLookupOf<T>, &BalanceOf<T>)>
    for WeightForSetTemporaryBalance<T>
{
    fn pays_fee(&self, target: (&AccountIdLookupOf<T>, &BalanceOf<T>)) -> Pays {
        if *target.1 > <BalanceOf<T>>::from(1000u32) {
            Pays::Yes
        } else {
            Pays::No
        }
    }
}

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
        // `on_initialize` is executed at the beginning of the block before any extrinsics are
        // dispatched.
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

        // `on_finalize` is executed at the end of block after all extrinsics are dispatched.
        fn on_finalize(_n: BlockNumberFor<T>) {
            // Perform necessary data/state clean up here.
            Self::check_win_condition();
        }
    }

    #[pallet::call(weight(<T as Config>::WeightInfo))]
    impl<T: Config> Pallet<T> {
        /// This is your public interface. Be extremely careful.
        /// This is just a simple example of how to interact with the pallet from the external
        /// world.
        /// Increasing the value of `TemporaryBalance` by `increase_by`.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::accumulate_temporary_balance())]
        pub fn accumulate_temporary_balance(
            who: OriginFor<T>,
            increase_by: T::Balance,
        ) -> DispatchResult {
            Self::do_accumulate_temporary_balance(who, increase_by)
        }

        /// A privileged call; in this case it resets our TemporaryBalance value to something new.
        // Implementation of a privileged call. The `origin` parameter is ROOT because
        // it's not (directly) from an extrinsic, but rather the system as a whole has decided
        // to execute it. Different runtimes have different reasons for allow privileged
        // calls to be executed - we don't need to care why. Because it's privileged, we can
        // assume it's a one-off operation and substantial processing/storage/memory can be used
        // without worrying about gameability or attack scenarios.
        //
        // The weight for this extrinsic we use our own weight object `WeightForSetTemporaryBalance` to
        // determine its weight
        #[pallet::call_index(1)]
        #[pallet::weight(WeightForSetTemporaryBalance::<T>(<BalanceOf<T>>::from(100u32)))]
        pub fn set_temporary_balance(
            origin: OriginFor<T>,
            who: AccountIdLookupOf<T>,
            #[pallet::compact] new_value: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let who = T::Lookup::lookup(who)?;

            let set_temporary_balance_key: u32 = 1;

            // Assert no value exists for the user.
            ensure!(
                TemporaryBalance::<T>::get(who.clone()).is_none(),
                Error::<T>::ValueAlreadySet
            );

            // Print out log or debug message in the console via log::{error, warn, info, debug,
            // trace}, accepting format strings similar to `println!`.
            // https://paritytech.github.io/substrate/master/sp_io/logging/fn.log.html
            // https://paritytech.github.io/substrate/master/frame_support/constant.LOG_TARGET.html
            info!("New value is now: {:?}", new_value);

            // Put the new value into storage.
            <TemporaryBalance<T>>::insert(who, new_value);
            Self::record_operation(set_temporary_balance_key)?;

            Self::deposit_event(Event::SetTemporaryBalance { balance: new_value });

            // All good, no refund.
            Ok(())
        }

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

    // The build of genesis for the pallet.
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

        // Check if the sender already has an entry in the TemporaryBalance map.
        // We want to ensure the sender is the owner of the entry.
        let current_temporary_balance = TemporaryBalance::<T>::get(&who);

        if current_temporary_balance.is_none() {
            // If no entry exists, we can create a new one or return an error
            // indicating the sender does not own an entry.
            return Err(Error::<T>::NoEntryForSender.into());
        }

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

    // Each operation has a specific key..
    // This should be a private function that records main extrinsic
    // operatrions.
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

    /// Checks if a player has won the game based on specific criteria.
    /// Checks if any player has reached the magic number in their balance or if the total balance
    /// reaches the magic number, and emits events accordingly.
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
