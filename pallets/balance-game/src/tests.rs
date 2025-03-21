use crate::{mock::*, Error, Event, *};
use frame_support::traits::OnFinalize;
use frame_support::{assert_noop, assert_ok};
use frame_system::pallet_prelude::BlockNumberFor;

#[test]
fn set_temporary_balance_works() {
    new_test_ext().execute_with(|| {
        // Ensure that setting a temporary balance value works
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            100
        ));
        assert_eq!(TemporaryBalance::<Test>::get(1), Some(100));

        // Ensure that the event is emitted
        System::assert_last_event(Event::SetTemporaryBalance { balance: 100 }.into());
    });
}

#[test]
fn set_dummy_fails_if_value_already_set() {
    new_test_ext().execute_with(|| {
        // Set a temporary value for account 1
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            100
        ));

        // Ensure that setting a dummy value again fails
        assert_noop!(
            BalanceGame::set_temporary_balance(RuntimeOrigin::root(), 1, 200),
            Error::<Test>::ValueAlreadySet
        );
    });
}

#[test]
fn accumulate_dummy_works() {
    new_test_ext().execute_with(|| {
        // Set a temporary balance for account 1
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            100
        ));

        // Accumulate temporary balance
        assert_ok!(BalanceGame::accumulate_temporary_balance(
            RuntimeOrigin::signed(1),
            50
        ));
        assert_eq!(TemporaryBalance::<Test>::get(1), Some(150));

        System::assert_last_event(Event::AccumulateTemporaryBalance { balance: 50 }.into());
    });
}

#[test]
fn accumulate_temporary_balance_fails_if_no_entry_for_sender() {
    new_test_ext().execute_with(|| {
        // Ensure that accumulating temporary balance fails if no entry exists for the sender
        assert_noop!(
            BalanceGame::accumulate_temporary_balance(RuntimeOrigin::signed(1), 50),
            Error::<Test>::NoEntryForSender
        );
    });
}

#[test]
fn update_balance_works() {
    new_test_ext().execute_with(|| {
        // Set temporary balance for account 1
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            100
        ));

        // Update the balance
        assert_ok!(BalanceGame::update_balance(RuntimeOrigin::signed(1)));
        assert_eq!(UserBalances::<Test>::get(1), Some(100));
        assert_eq!(TotalBalance::<Test>::get(), 100);

        // Ensure that the event is emitted
        System::assert_last_event(Event::BalanceUpdated(1, 100).into());
    });
}

#[test]
fn update_balance_fails_if_temporary_balance_not_found() {
    new_test_ext().execute_with(|| {
        // Ensure that updating the balance fails if no temporary balance exists
        assert_noop!(
            BalanceGame::update_balance(RuntimeOrigin::signed(1)),
            Error::<Test>::TempoaryBalanceNotFound
        );
    });
}

#[test]
fn clear_temporary_balance_works() {
    new_test_ext().execute_with(|| {
        // Set temporary balance for account 1

        // Clear the temporary balance
        assert_ok!(BalanceGame::clear_temporary_balance(
            RuntimeOrigin::signed(1),
            1
        ));
        assert_eq!(TemporaryBalance::<Test>::get(1), None);
    });
}

#[test]
fn check_win_condition_in_on_finalize_works() {
    new_test_ext().execute_with(|| {
        // Set temporary balance for account 1
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            1_000_000_000
        ));

        // Update user balance to trigger the win condition
        assert_ok!(BalanceGame::update_balance(RuntimeOrigin::signed(1)));

        // Simulate the end of the block to trigger `on_finalize`
        BalanceGame::on_finalize(1);

        // Ensure that the win condition is checked and the event is emitted
        System::assert_last_event(Event::GameWon(1, 1_000_000_000).into());
    });
}

#[test]
fn operation_limit_exceeded() {
    new_test_ext().execute_with(|| {
        // Set temporary balance for account 1
        assert_ok!(BalanceGame::set_temporary_balance(
            RuntimeOrigin::root(),
            1,
            100
        ));

        // Perform operations until the limit is reached
        for _ in 0..31 {
            assert_ok!(BalanceGame::accumulate_temporary_balance(
                RuntimeOrigin::signed(1),
                10
            ));
        }

        // Ensure that the next operation fails due to the operation limit
        assert_noop!(
            BalanceGame::accumulate_temporary_balance(RuntimeOrigin::signed(1), 10),
            Error::<Test>::OperationLimitExceeded
        );
    });
}
