extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke},
    token, vec, Address, Env, IntoVal, Vec,
};

const C: i128 = 100;
const PERIOD: u64 = 1_000;
const GRACE: u64 = 100;

struct World {
    env: Env,
    client: AjoCircleClient<'static>,
    token: Address,
    sac: token::StellarAssetClient<'static>,
    organizer: Address,
    members: Vec<Address>,
}

impl World {
    fn new(n: u32) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(10_000);
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(admin).address();
        let sac = token::StellarAssetClient::new(&env, &token);
        let contract = env.register(AjoCircle, ());
        let client = AjoCircleClient::new(&env, &contract);
        let organizer = Address::generate(&env);
        let mut members = Vec::new(&env);
        for _ in 0..n {
            let m = Address::generate(&env);
            sac.mint(&m, &10_000);
            members.push_back(m);
        }
        World {
            env,
            client,
            token,
            sac,
            organizer,
            members,
        }
    }

    fn create(&self) -> u32 {
        self.client.create_circle(
            &self.organizer,
            &self.token,
            &self.members,
            &C,
            &PERIOD,
            &GRACE,
        )
    }

    fn active(&self) -> u32 {
        let id = self.create();
        for m in self.members.iter() {
            self.client.join(&id, &m);
        }
        id
    }

    fn bal(&self, a: &Address) -> i128 {
        token::Client::new(&self.env, &self.token).balance(a)
    }

    fn at(&self, t: u64) {
        self.env.ledger().set_timestamp(t);
    }

    fn m(&self, i: u32) -> Address {
        self.members.get_unchecked(i)
    }
}

#[test]
fn create_returns_sequential_ids_and_stores_circle() {
    let w = World::new(3);
    assert_eq!(w.create(), 0);
    assert_eq!(w.create(), 1);
    assert_eq!(w.client.circle_count(), 2);
    let c = w.client.get_circle(&0);
    assert_eq!(c.state, State::Forming);
    assert_eq!(c.contribution, C);
    assert_eq!(w.client.required_deposit(&0), 2 * C);
}

#[test]
fn create_rejects_invalid_inputs() {
    let w = World::new(3);
    let one = vec![&w.env, w.m(0)];
    let dup = vec![&w.env, w.m(0), w.m(0)];
    let mut many = Vec::new(&w.env);
    for _ in 0..21 {
        many.push_back(Address::generate(&w.env));
    }
    let org = &w.organizer;
    let t = &w.token;
    assert_eq!(
        w.client
            .try_create_circle(org, t, &one, &C, &PERIOD, &GRACE),
        Err(Ok(Error::InvalidMembers))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &dup, &C, &PERIOD, &GRACE),
        Err(Ok(Error::InvalidMembers))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &many, &C, &PERIOD, &GRACE),
        Err(Ok(Error::InvalidMembers))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &0, &PERIOD, &GRACE),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &-5, &PERIOD, &GRACE),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &C, &(MIN_PERIOD - 1), &0),
        Err(Ok(Error::InvalidPeriod))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &C, &(MAX_PERIOD + 1), &0),
        Err(Ok(Error::InvalidPeriod))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &C, &PERIOD, &(PERIOD + 1)),
        Err(Ok(Error::InvalidGrace))
    );
    assert_eq!(
        w.client
            .try_create_circle(org, t, &w.members, &i128::MAX, &PERIOD, &GRACE),
        Err(Ok(Error::Overflow))
    );
    assert_eq!(w.client.circle_count(), 0);
}

#[test]
fn boundary_member_counts_are_accepted() {
    let w = World::new(2);
    assert_eq!(w.create(), 0);
    let mut twenty = Vec::new(&w.env);
    for _ in 0..20 {
        twenty.push_back(Address::generate(&w.env));
    }
    w.client
        .create_circle(&w.organizer, &w.token, &twenty, &C, &PERIOD, &GRACE);
    assert_eq!(w.client.required_deposit(&1), 19 * C);
}

#[test]
fn join_locks_deposit_and_last_join_starts_circle() {
    let w = World::new(3);
    let id = w.create();
    w.client.join(&id, &w.m(0));
    assert_eq!(w.bal(&w.m(0)), 10_000 - 2 * C);
    assert_eq!(w.client.deposit_of(&id, &w.m(0)), 2 * C);
    assert_eq!(w.client.get_circle(&id).state, State::Forming);
    w.client.join(&id, &w.m(1));
    w.client.join(&id, &w.m(2));
    let c = w.client.get_circle(&id);
    assert_eq!(c.state, State::Active);
    assert_eq!(c.start_time, 10_000);
    assert_eq!(w.bal(&w.client.address), 3 * 2 * C);
}

#[test]
fn join_rejects_outsider_duplicate_and_wrong_state() {
    let w = World::new(2);
    let id = w.create();
    let outsider = Address::generate(&w.env);
    w.sac.mint(&outsider, &1_000);
    assert_eq!(w.client.try_join(&id, &outsider), Err(Ok(Error::NotMember)));
    w.client.join(&id, &w.m(0));
    assert_eq!(
        w.client.try_join(&id, &w.m(0)),
        Err(Ok(Error::AlreadyJoined))
    );
    w.client.join(&id, &w.m(1));
    assert_eq!(w.client.try_join(&id, &w.m(1)), Err(Ok(Error::NotForming)));
    assert_eq!(
        w.client.try_join(&99, &w.m(0)),
        Err(Ok(Error::CircleNotFound))
    );
}

#[test]
fn cancel_refunds_joined_members_only_via_claim() {
    let w = World::new(3);
    let id = w.create();
    w.client.join(&id, &w.m(0));
    w.client.cancel(&id);
    assert_eq!(w.client.get_circle(&id).state, State::Cancelled);
    assert_eq!(w.client.claimable_of(&id, &w.m(0)), 2 * C);
    assert_eq!(w.client.claimable_of(&id, &w.m(1)), 0);
    assert_eq!(w.client.claim(&id, &w.m(0)), 2 * C);
    assert_eq!(w.bal(&w.m(0)), 10_000);
    assert_eq!(
        w.client.try_claim(&id, &w.m(0)),
        Err(Ok(Error::NothingToClaim))
    );
    assert_eq!(w.client.try_join(&id, &w.m(1)), Err(Ok(Error::NotForming)));
    assert_eq!(w.client.try_cancel(&id), Err(Ok(Error::NotForming)));
}

#[test]
fn cannot_cancel_active_circle() {
    let w = World::new(2);
    let id = w.active();
    assert_eq!(w.client.try_cancel(&id), Err(Ok(Error::NotForming)));
}

#[test]
fn full_happy_path_pays_every_member_and_returns_deposits() {
    let w = World::new(3);
    let id = w.active();
    for r in 0..3u32 {
        w.at(10_000 + r as u64 * PERIOD);
        for i in 0..3u32 {
            if i != r {
                w.client.contribute(&id, &w.m(i));
                assert!(w.client.has_paid(&id, &r, &w.m(i)));
            }
        }
        w.client.settle_round(&id);
        if r == 0 {
            assert_eq!(w.client.claimable_of(&id, &w.m(0)), 2 * C);
        }
    }
    assert_eq!(w.client.get_circle(&id).state, State::Completed);
    for i in 0..3u32 {
        w.client.claim(&id, &w.m(i));
        // Paid 2C in, received 2C pot and got the full 2C deposit back: net zero.
        assert_eq!(w.bal(&w.m(i)), 10_000);
    }
    assert_eq!(w.bal(&w.client.address), 0);
}

#[test]
fn default_is_slashed_and_recipient_still_paid_in_full() {
    let w = World::new(3);
    let id = w.active();
    // Round 0: recipient m0. m1 pays, m2 defaults.
    w.client.contribute(&id, &w.m(1));
    w.at(10_000 + PERIOD + GRACE);
    w.client.settle_round(&id);
    assert_eq!(w.client.claimable_of(&id, &w.m(0)), 2 * C);
    assert_eq!(w.client.deposit_of(&id, &w.m(2)), C);
    assert_eq!(w.client.deposit_of(&id, &w.m(1)), 2 * C);
    assert_eq!(w.client.get_circle(&id).round, 1);
    // Defaulter can still pay in later rounds.
    w.client.contribute(&id, &w.m(2));
}

#[test]
fn defaulter_loses_whole_deposit_but_pot_stays_funded_through_completion() {
    let w = World::new(3);
    let id = w.active();
    // m2 never pays anything. Rounds 0 and 1 are the ones m2 owes.
    for r in 0..3u32 {
        w.at(10_000 + (r as u64 + 1) * PERIOD + GRACE);
        for i in 0..2u32 {
            if i != r {
                w.client.contribute(&id, &w.m(i));
            }
        }
        w.client.settle_round(&id);
    }
    assert_eq!(w.client.get_circle(&id).state, State::Completed);
    assert_eq!(w.client.deposit_of(&id, &w.m(2)), 0);
    for i in 0..3u32 {
        let amt = w.client.claimable_of(&id, &w.m(i));
        if amt > 0 {
            w.client.claim(&id, &w.m(i));
        }
    }
    // m2 paid nothing, lost its 2C deposit and received a 2C pot: net 0 for m2, while
    // m0 and m1 each paid what they owed and received a full pot.
    assert_eq!(w.bal(&w.m(2)), 10_000);
    assert_eq!(w.bal(&w.m(0)), 10_000);
    assert_eq!(w.bal(&w.m(1)), 10_000);
    assert_eq!(w.bal(&w.client.address), 0);
}

#[test]
fn contribute_rejections() {
    let w = World::new(3);
    let id = w.create();
    assert_eq!(
        w.client.try_contribute(&id, &w.m(1)),
        Err(Ok(Error::NotActive))
    );
    let id = w.active();
    let outsider = Address::generate(&w.env);
    assert_eq!(
        w.client.try_contribute(&id, &outsider),
        Err(Ok(Error::NotMember))
    );
    assert_eq!(
        w.client.try_contribute(&id, &w.m(0)),
        Err(Ok(Error::RecipientDoesNotPay))
    );
    w.client.contribute(&id, &w.m(1));
    assert_eq!(
        w.client.try_contribute(&id, &w.m(1)),
        Err(Ok(Error::AlreadyPaid))
    );
}

#[test]
fn next_round_is_not_open_early() {
    let w = World::new(2);
    let id = w.active();
    w.client.contribute(&id, &w.m(1));
    w.client.settle_round(&id); // all paid: settles immediately, still at t=10_000
                                // Round 1 opens at start + period; m0 owes m1.
    assert_eq!(
        w.client.try_contribute(&id, &w.m(0)),
        Err(Ok(Error::RoundNotOpen))
    );
    w.at(10_000 + PERIOD);
    w.client.contribute(&id, &w.m(0));
}

#[test]
fn settle_rejects_early_and_wrong_state() {
    let w = World::new(3);
    let id = w.create();
    assert_eq!(w.client.try_settle_round(&id), Err(Ok(Error::NotActive)));
    let id = w.active();
    w.client.contribute(&id, &w.m(1));
    assert_eq!(
        w.client.try_settle_round(&id),
        Err(Ok(Error::RoundNotSettleable))
    );
    w.at(10_000 + PERIOD + GRACE - 1);
    assert_eq!(
        w.client.try_settle_round(&id),
        Err(Ok(Error::RoundNotSettleable))
    );
    w.at(10_000 + PERIOD + GRACE);
    w.client.settle_round(&id);
}

#[test]
fn cannot_settle_completed_circle_twice() {
    let w = World::new(2);
    let id = w.active();
    w.client.contribute(&id, &w.m(1));
    w.client.settle_round(&id);
    w.at(10_000 + PERIOD);
    w.client.contribute(&id, &w.m(0));
    w.client.settle_round(&id);
    assert_eq!(w.client.get_circle(&id).state, State::Completed);
    assert_eq!(w.client.try_settle_round(&id), Err(Ok(Error::NotActive)));
    assert_eq!(
        w.client.try_contribute(&id, &w.m(1)),
        Err(Ok(Error::NotActive))
    );
}

#[test]
fn claim_is_idempotent_and_rejects_empty() {
    let w = World::new(2);
    let id = w.active();
    w.client.contribute(&id, &w.m(1));
    w.client.settle_round(&id);
    assert_eq!(w.client.claim(&id, &w.m(0)), C);
    assert_eq!(
        w.client.try_claim(&id, &w.m(0)),
        Err(Ok(Error::NothingToClaim))
    );
    let outsider = Address::generate(&w.env);
    assert_eq!(
        w.client.try_claim(&id, &outsider),
        Err(Ok(Error::NothingToClaim))
    );
}

#[test]
fn circles_are_isolated_from_each_other() {
    let w = World::new(2);
    let a = w.active();
    let b = w.active();
    w.client.contribute(&a, &w.m(1));
    assert!(w.client.has_paid(&a, &0, &w.m(1)));
    assert!(!w.client.has_paid(&b, &0, &w.m(1)));
    w.client.settle_round(&a);
    assert_eq!(w.client.claimable_of(&b, &w.m(0)), 0);
}

#[test]
fn events_are_emitted() {
    let w = World::new(2);
    let id = w.active();
    w.client.contribute(&id, &w.m(1));
    w.client.settle_round(&id);
    let evs = w.env.events().all().filter_by_contract(&w.client.address);
    assert_eq!(evs.events().len(), 1); // RoundSettled from the last call
}

// --- authorization: no mock_all_auths, explicit signers only ---

fn auth_world() -> (Env, AjoCircleClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.ledger().set_timestamp(10_000);
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin).address();
    let contract = env.register(AjoCircle, ());
    let client = AjoCircleClient::new(&env, &contract);
    let organizer = Address::generate(&env);
    let a = Address::generate(&env);
    (env, client, token, organizer, a)
}

#[test]
fn create_requires_organizer_auth() {
    let (env, client, token, organizer, a) = auth_world();
    let b = Address::generate(&env);
    let members = vec![&env, a, b];
    let stranger = Address::generate(&env);
    // Authorised by a stranger, not the organizer.
    let res = client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "create_circle",
                args: (&organizer, &token, &members, &C, &PERIOD, &GRACE).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_create_circle(&organizer, &token, &members, &C, &PERIOD, &GRACE);
    assert!(res.is_err());
    // And with the organizer's own auth it works.
    client
        .mock_auths(&[MockAuth {
            address: &organizer,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "create_circle",
                args: (&organizer, &token, &members, &C, &PERIOD, &GRACE).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .create_circle(&organizer, &token, &members, &C, &PERIOD, &GRACE);
}

#[test]
fn cancel_requires_organizer_not_member() {
    let w = World::new(2);
    let id = w.create();
    w.env.set_auths(&[]);
    // With auth reset, a call carrying no authorisation must fail.
    assert!(w.client.try_cancel(&id).is_err());
    let _ = &w.sac;
}

#[test]
fn member_actions_fail_without_that_members_auth() {
    let w = World::new(2);
    let id = w.create();
    w.env.set_auths(&[]);
    assert!(w.client.try_join(&id, &w.m(0)).is_err());
    assert!(w.client.try_contribute(&id, &w.m(1)).is_err());
    assert!(w.client.try_claim(&id, &w.m(0)).is_err());
}
