//! Ajo Circle: collateralised rotating savings circles (ROSCA / ajo / susu / chama).
//!
//! `n` members each pay `contribution` per round. In round `r` the pot goes to
//! `members[r]`; the recipient does not pay in their own round, so every member owes
//! exactly `n - 1` payments. Each member locks a deposit of `contribution * (n - 1)`
//! when joining. If a member misses a round, `settle_round` slashes one `contribution`
//! from their deposit, so the pot is always paid in full and a default can never
//! leave a recipient short. Unused deposit is returned when the circle completes.
//!
//! All outflows are pull-based (`claim`), so one member's token trustline problem can
//! never stall a round for everyone else. The contract has no admin and no upgrade
//! path; the organizer can only cancel a circle that has not started.
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, Vec,
};

pub const MIN_MEMBERS: u32 = 2;
pub const MAX_MEMBERS: u32 = 20;
pub const MIN_PERIOD: u64 = 60;
pub const MAX_PERIOD: u64 = 366 * 24 * 3600;

const DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 30 * DAY_LEDGERS;
const TTL_EXTEND_TO: u32 = 120 * DAY_LEDGERS;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    CircleNotFound = 1,
    InvalidMembers = 2,
    InvalidAmount = 3,
    InvalidPeriod = 4,
    InvalidGrace = 5,
    NotMember = 6,
    AlreadyJoined = 7,
    NotForming = 8,
    NotActive = 9,
    NotOrganizer = 10,
    RoundNotOpen = 11,
    AlreadyPaid = 12,
    RecipientDoesNotPay = 13,
    RoundNotSettleable = 14,
    NothingToClaim = 15,
    Overflow = 16,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    /// Waiting for every listed member to lock their deposit.
    Forming,
    /// All members joined; rounds are running.
    Active,
    /// Every round settled; leftover deposits are claimable.
    Completed,
    /// Cancelled while forming; deposits are claimable.
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Circle {
    pub organizer: Address,
    pub token: Address,
    /// Payout order: `members[r]` receives the pot of round `r`.
    pub members: Vec<Address>,
    pub contribution: i128,
    /// Seconds per round.
    pub period: u64,
    /// Extra seconds after a round's deadline before missing payments are slashed.
    pub grace: u64,
    pub state: State,
    pub joined: u32,
    pub start_time: u64,
    /// Index of the round currently collecting payments.
    pub round: u32,
    /// Payments received in the current round.
    pub paid: u32,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Count,
    Circle(u32),
    Joined(u32, Address),
    Deposit(u32, Address),
    Paid(u32, u32, Address),
    Claimable(u32, Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircleCreated {
    #[topic]
    pub circle_id: u32,
    pub organizer: Address,
    pub token: Address,
    pub contribution: i128,
    pub members: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Joined {
    #[topic]
    pub circle_id: u32,
    pub member: Address,
    pub deposit: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Started {
    #[topic]
    pub circle_id: u32,
    pub start_time: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contributed {
    #[topic]
    pub circle_id: u32,
    #[topic]
    pub round: u32,
    pub member: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slashed {
    #[topic]
    pub circle_id: u32,
    #[topic]
    pub round: u32,
    pub member: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoundSettled {
    #[topic]
    pub circle_id: u32,
    #[topic]
    pub round: u32,
    pub recipient: Address,
    pub pot: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Completed {
    #[topic]
    pub circle_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
    #[topic]
    pub circle_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claimed {
    #[topic]
    pub circle_id: u32,
    pub member: Address,
    pub amount: i128,
}

fn load(env: &Env, id: u32) -> Result<Circle, Error> {
    let key = Key::Circle(id);
    let circle: Circle = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::CircleNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    Ok(circle)
}

fn save(env: &Env, id: u32, circle: &Circle) {
    let key = Key::Circle(id);
    env.storage().persistent().set(&key, circle);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn get_i128(env: &Env, key: &Key) -> i128 {
    env.storage().persistent().get(key).unwrap_or(0)
}

fn set_i128(env: &Env, key: &Key, value: i128) {
    if value == 0 {
        env.storage().persistent().remove(key);
    } else {
        env.storage().persistent().set(key, &value);
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

fn add_claimable(env: &Env, id: u32, member: &Address, amount: i128) -> Result<(), Error> {
    if amount == 0 {
        return Ok(());
    }
    let key = Key::Claimable(id, member.clone());
    let next = get_i128(env, &key)
        .checked_add(amount)
        .ok_or(Error::Overflow)?;
    set_i128(env, &key, next);
    Ok(())
}

fn is_member(circle: &Circle, who: &Address) -> bool {
    circle.members.iter().any(|m| m == *who)
}

/// Deposit every member must lock: enough to cover all `n - 1` payments they owe.
fn required_deposit(circle: &Circle) -> Result<i128, Error> {
    circle
        .contribution
        .checked_mul((circle.members.len() - 1) as i128)
        .ok_or(Error::Overflow)
}

fn round_open_time(circle: &Circle) -> u64 {
    circle.start_time + (circle.round as u64) * circle.period
}

fn round_deadline(circle: &Circle) -> u64 {
    circle.start_time + (circle.round as u64 + 1) * circle.period
}

#[contract]
pub struct AjoCircle;

#[contractimpl]
impl AjoCircle {
    /// Create a circle. `members` fixes the payout order and must be unique.
    /// Returns the circle id. The organizer does not have to be a member.
    pub fn create_circle(
        env: Env,
        organizer: Address,
        token: Address,
        members: Vec<Address>,
        contribution: i128,
        period: u64,
        grace: u64,
    ) -> Result<u32, Error> {
        organizer.require_auth();
        let n = members.len();
        if !(MIN_MEMBERS..=MAX_MEMBERS).contains(&n) {
            return Err(Error::InvalidMembers);
        }
        for i in 0..n {
            for j in (i + 1)..n {
                if members.get_unchecked(i) == members.get_unchecked(j) {
                    return Err(Error::InvalidMembers);
                }
            }
        }
        if contribution <= 0 {
            return Err(Error::InvalidAmount);
        }
        if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
            return Err(Error::InvalidPeriod);
        }
        if grace > period {
            return Err(Error::InvalidGrace);
        }
        let circle = Circle {
            organizer: organizer.clone(),
            token: token.clone(),
            members,
            contribution,
            period,
            grace,
            state: State::Forming,
            joined: 0,
            start_time: 0,
            round: 0,
            paid: 0,
        };
        // Reject amounts whose total could overflow before any state is written.
        let deposit = required_deposit(&circle)?;
        deposit.checked_mul(n as i128).ok_or(Error::Overflow)?;

        let id: u32 = env.storage().instance().get(&Key::Count).unwrap_or(0);
        env.storage().instance().set(&Key::Count, &(id + 1));
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
        save(&env, id, &circle);
        CircleCreated {
            circle_id: id,
            organizer,
            token,
            contribution,
            members: n,
        }
        .publish(&env);
        Ok(id)
    }

    /// A listed member locks their deposit. The last member to join starts the circle.
    pub fn join(env: Env, circle_id: u32, member: Address) -> Result<(), Error> {
        member.require_auth();
        let mut circle = load(&env, circle_id)?;
        if circle.state != State::Forming {
            return Err(Error::NotForming);
        }
        if !is_member(&circle, &member) {
            return Err(Error::NotMember);
        }
        let joined_key = Key::Joined(circle_id, member.clone());
        if env.storage().persistent().has(&joined_key) {
            return Err(Error::AlreadyJoined);
        }
        let deposit = required_deposit(&circle)?;
        token::Client::new(&env, &circle.token).transfer(
            &member,
            env.current_contract_address(),
            &deposit,
        );
        env.storage().persistent().set(&joined_key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&joined_key, TTL_THRESHOLD, TTL_EXTEND_TO);
        set_i128(&env, &Key::Deposit(circle_id, member.clone()), deposit);
        circle.joined += 1;
        Joined {
            circle_id,
            member,
            deposit,
        }
        .publish(&env);
        if circle.joined == circle.members.len() {
            circle.state = State::Active;
            circle.start_time = env.ledger().timestamp();
            Started {
                circle_id,
                start_time: circle.start_time,
            }
            .publish(&env);
        }
        save(&env, circle_id, &circle);
        Ok(())
    }

    /// The organizer cancels a circle that has not started; joined members can claim
    /// their deposits back.
    pub fn cancel(env: Env, circle_id: u32) -> Result<(), Error> {
        let mut circle = load(&env, circle_id)?;
        circle.organizer.require_auth();
        if circle.state != State::Forming {
            return Err(Error::NotForming);
        }
        for m in circle.members.iter() {
            let key = Key::Deposit(circle_id, m.clone());
            let dep = get_i128(&env, &key);
            if dep > 0 {
                set_i128(&env, &key, 0);
                add_claimable(&env, circle_id, &m, dep)?;
            }
        }
        circle.state = State::Cancelled;
        save(&env, circle_id, &circle);
        Cancelled { circle_id }.publish(&env);
        Ok(())
    }

    /// Pay this round's contribution. The round's recipient does not pay.
    pub fn contribute(env: Env, circle_id: u32, member: Address) -> Result<(), Error> {
        member.require_auth();
        let mut circle = load(&env, circle_id)?;
        if circle.state != State::Active {
            return Err(Error::NotActive);
        }
        if !is_member(&circle, &member) {
            return Err(Error::NotMember);
        }
        if env.ledger().timestamp() < round_open_time(&circle) {
            return Err(Error::RoundNotOpen);
        }
        let round = circle.round;
        if circle.members.get_unchecked(round) == member {
            return Err(Error::RecipientDoesNotPay);
        }
        let paid_key = Key::Paid(circle_id, round, member.clone());
        if env.storage().persistent().has(&paid_key) {
            return Err(Error::AlreadyPaid);
        }
        token::Client::new(&env, &circle.token).transfer(
            &member,
            env.current_contract_address(),
            &circle.contribution,
        );
        env.storage().persistent().set(&paid_key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&paid_key, TTL_THRESHOLD, TTL_EXTEND_TO);
        circle.paid += 1;
        save(&env, circle_id, &circle);
        Contributed {
            circle_id,
            round,
            member,
            amount: circle.contribution,
        }
        .publish(&env);
        Ok(())
    }

    /// Close the current round and credit the pot to the recipient. Callable by anyone
    /// once every payer has paid, or once the deadline plus grace has passed; members
    /// who have not paid by then are slashed one contribution from their deposit.
    pub fn settle_round(env: Env, circle_id: u32) -> Result<(), Error> {
        let mut circle = load(&env, circle_id)?;
        if circle.state != State::Active {
            return Err(Error::NotActive);
        }
        let n = circle.members.len();
        let all_paid = circle.paid == n - 1;
        let cutoff = round_deadline(&circle) + circle.grace;
        if !all_paid && env.ledger().timestamp() < cutoff {
            return Err(Error::RoundNotSettleable);
        }
        let round = circle.round;
        let recipient = circle.members.get_unchecked(round);
        if !all_paid {
            for m in circle.members.iter() {
                if m == recipient
                    || env
                        .storage()
                        .persistent()
                        .has(&Key::Paid(circle_id, round, m.clone()))
                {
                    continue;
                }
                let dep_key = Key::Deposit(circle_id, m.clone());
                // The deposit covers all n - 1 payments, so this never underflows.
                set_i128(
                    &env,
                    &dep_key,
                    get_i128(&env, &dep_key) - circle.contribution,
                );
                Slashed {
                    circle_id,
                    round,
                    member: m,
                    amount: circle.contribution,
                }
                .publish(&env);
            }
        }
        let pot = circle
            .contribution
            .checked_mul((n - 1) as i128)
            .ok_or(Error::Overflow)?;
        add_claimable(&env, circle_id, &recipient, pot)?;
        RoundSettled {
            circle_id,
            round,
            recipient,
            pot,
        }
        .publish(&env);

        circle.round += 1;
        circle.paid = 0;
        if circle.round == n {
            circle.state = State::Completed;
            for m in circle.members.iter() {
                let key = Key::Deposit(circle_id, m.clone());
                let left = get_i128(&env, &key);
                if left > 0 {
                    set_i128(&env, &key, 0);
                    add_claimable(&env, circle_id, &m, left)?;
                }
            }
            Completed { circle_id }.publish(&env);
        }
        save(&env, circle_id, &circle);
        Ok(())
    }

    /// Withdraw everything credited to `member` (pots, leftover or refunded deposits).
    pub fn claim(env: Env, circle_id: u32, member: Address) -> Result<i128, Error> {
        member.require_auth();
        let circle = load(&env, circle_id)?;
        let key = Key::Claimable(circle_id, member.clone());
        let amount = get_i128(&env, &key);
        if amount <= 0 {
            return Err(Error::NothingToClaim);
        }
        set_i128(&env, &key, 0);
        token::Client::new(&env, &circle.token).transfer(
            &env.current_contract_address(),
            &member,
            &amount,
        );
        Claimed {
            circle_id,
            member,
            amount,
        }
        .publish(&env);
        Ok(amount)
    }

    pub fn get_circle(env: Env, circle_id: u32) -> Result<Circle, Error> {
        load(&env, circle_id)
    }

    pub fn circle_count(env: Env) -> u32 {
        env.storage().instance().get(&Key::Count).unwrap_or(0)
    }

    pub fn deposit_of(env: Env, circle_id: u32, member: Address) -> i128 {
        get_i128(&env, &Key::Deposit(circle_id, member))
    }

    pub fn claimable_of(env: Env, circle_id: u32, member: Address) -> i128 {
        get_i128(&env, &Key::Claimable(circle_id, member))
    }

    pub fn has_paid(env: Env, circle_id: u32, round: u32, member: Address) -> bool {
        env.storage()
            .persistent()
            .has(&Key::Paid(circle_id, round, member))
    }

    pub fn required_deposit(env: Env, circle_id: u32) -> Result<i128, Error> {
        required_deposit(&load(&env, circle_id)?)
    }
}

#[cfg(test)]
mod test;
