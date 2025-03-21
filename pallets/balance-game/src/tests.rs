use crate::{mock::*, Error, Event, *};
use frame_support::traits::OnFinalize;
use frame_support::{assert_noop, assert_ok};

#[test]
fn accumulate_temporary_balance_works() {
    new_test_ext().execute_with(|| {
        // Accumulate temporary balance
        assert_ok!(BalanceGame::accumulate_temporary_balance(
            RuntimeOrigin::signed(1),
            150
        ));
        assert_eq!(TemporaryBalance::<Test>::get(1), Some(150));

        System::assert_last_event(Event::OperationCountUpdated(0, 1).into());
    });
}

#[test]
fn update_balance_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(BalanceGame::accumulate_temporary_balance(
            RuntimeOrigin::signed(1),
            150
        ));
        // Update the balance
        assert_ok!(BalanceGame::update_balance(RuntimeOrigin::signed(1)));
        assert_eq!(UserBalances::<Test>::get(1), Some(150));
        assert_eq!(TotalBalance::<Test>::get(), 150);

        // Ensure that the event is emitted
        System::assert_last_event(Event::BalanceUpdated(1, 150).into());
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
        assert_ok!(BalanceGame::accumulate_temporary_balance(
            RuntimeOrigin::signed(1),
            150
        ));

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
        assert_ok!(BalanceGame::accumulate_temporary_balance(
            RuntimeOrigin::signed(1),
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
