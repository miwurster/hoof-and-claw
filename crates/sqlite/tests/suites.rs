//! The SQLite adapter passes the shared suite of every store port, each test on its own fresh database file.

use sqlite::Database;

macro_rules! shared_suite {
    ($suite:path => $($case:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $case() {
                use $suite as suite;
                let dir = tempfile::tempdir().unwrap();
                let database = Database::open(&dir.path().join("game.db")).await.unwrap();
                suite::$case(database.clone()).await;
                database.close().await;
            }
        )+
    };
}

mod guessing_store {
    use super::*;

    shared_suite!(guessing::store::suite =>
        a_player_who_was_never_stored_stands_at_score_0_with_no_open_guess,
        a_placed_guess_is_the_players_open_guess,
        a_player_with_an_open_guess_cannot_place_another,
        players_do_not_affect_each_other,
        a_resolution_moves_the_score_and_becomes_the_last_resolved_guess,
        a_seed_price_without_a_trade_id_is_kept_as_one,
        a_score_can_go_below_0,
        a_resolution_saved_again_changes_nothing,
        after_a_resolution_the_player_can_place_the_next_guess,
        the_last_resolved_guess_is_the_most_recent_one,
        resolutions_saved_together_move_each_players_own_score,
        the_open_guesses_are_every_players_open_guess_and_no_resolved_one,
    );
}

mod identity_store {
    use super::*;

    shared_suite!(identity::store::suite =>
        a_token_without_a_guess_has_no_player,
        a_linked_token_belongs_to_its_player,
        a_token_linked_twice_keeps_the_player_linked_first,
        tokens_do_not_share_a_player,
    );
}
