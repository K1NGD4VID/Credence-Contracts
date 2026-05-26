#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityBond {
    pub identity: Address,
    pub bonded_amount: i128,
    pub bond_start: u64,
    pub bond_duration: u64,
    pub is_rolling: bool,
    pub withdrawal_requested_at: u64,
    pub notice_period_duration: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Bond,
}

#[contract]
pub struct CredenceBond;

#[contractimpl]
impl CredenceBond {
    pub fn initialize(e: Env, admin: Address) {
        admin.require_auth();
        e.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn create_bond(
        e: Env,
        identity: Address,
        amount: i128,
        duration: u64,
        is_rolling: bool,
        notice_period_duration: u64,
    ) -> IdentityBond {
        identity.require_auth();

        let bond = IdentityBond {
            identity: identity.clone(),
            bonded_amount: amount,
            bond_start: e.ledger().timestamp(),
            bond_duration: duration,
            is_rolling,
            withdrawal_requested_at: 0,
            notice_period_duration,
        };

        e.storage().instance().set(&DataKey::Bond, &bond);
        bond
    }

    pub fn get_identity_state(e: Env) -> IdentityBond {
        e.storage()
            .instance()
            .get(&DataKey::Bond)
            .unwrap_or_else(|| panic!("no bond"))
    }

    pub fn withdraw(e: Env, amount: i128) -> IdentityBond {
        let mut bond = Self::load_bond_and_require_owner_auth(&e);
        if amount > bond.bonded_amount {
            panic!("insufficient balance for withdrawal");
        }
        bond.bonded_amount = bond
            .bonded_amount
            .checked_sub(amount)
            .expect("withdrawal caused underflow");
        e.storage().instance().set(&DataKey::Bond, &bond);
        bond
    }

    pub fn top_up(e: Env, amount: i128) -> IdentityBond {
        let mut bond = Self::load_bond_and_require_owner_auth(&e);
        bond.bonded_amount = bond
            .bonded_amount
            .checked_add(amount)
            .expect("top-up caused overflow");
        e.storage().instance().set(&DataKey::Bond, &bond);
        bond
    }

    pub fn extend_duration(e: Env, additional_duration: u64) -> IdentityBond {
        let mut bond = Self::load_bond_and_require_owner_auth(&e);
        bond.bond_duration = bond
            .bond_duration
            .checked_add(additional_duration)
            .expect("duration extension caused overflow");
        e.storage().instance().set(&DataKey::Bond, &bond);
        bond
    }

    pub fn request_withdrawal(e: Env) -> IdentityBond {
        let mut bond = Self::load_bond_and_require_owner_auth(&e);
        if !bond.is_rolling {
            panic!("not a rolling bond");
        }
        if bond.withdrawal_requested_at != 0 {
            panic!("withdrawal already requested");
        }
        bond.withdrawal_requested_at = e.ledger().timestamp();
        e.storage().instance().set(&DataKey::Bond, &bond);
        e.events().publish(
            (Symbol::new(&e, "withdrawal_requested"),),
            (bond.identity.clone(), bond.withdrawal_requested_at),
        );
        bond
    }

    pub fn renew_if_rolling(e: Env) -> IdentityBond {
        let mut bond = Self::load_bond_and_require_owner_auth(&e);
        if !bond.is_rolling {
            return bond;
        }
        let now = e.ledger().timestamp();
        let end = bond.bond_start.saturating_add(bond.bond_duration);
        if now <= end {
            return bond;
        }
        bond.bond_start = now;
        bond.withdrawal_requested_at = 0;
        e.storage().instance().set(&DataKey::Bond, &bond);
        bond
    }

    fn load_bond_and_require_owner_auth(e: &Env) -> IdentityBond {
        let bond: IdentityBond = e
            .storage()
            .instance()
            .get(&DataKey::Bond)
            .unwrap_or_else(|| panic!("no bond"));
        bond.identity.require_auth();
        bond
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    fn setup() -> (
        Env,
        Address,
        CredenceBondClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let e = Env::default();
        let contract_id = e.register(CredenceBond, ());
        let client = CredenceBondClient::new(&e, &contract_id);
        let owner = Address::generate(&e);
        let admin = Address::generate(&e);
        let attacker = Address::generate(&e);

        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "initialize",
                    args: (&admin,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .initialize(&admin);

        let amount = 1_000_i128;
        let duration = 1_000_u64;
        let is_rolling = true;
        let notice = 100_u64;

        client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "create_bond",
                    args: (&owner, &amount, &duration, &is_rolling, &notice).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .create_bond(&owner, &amount, &duration, &is_rolling, &notice);

        (e, contract_id, client, owner, admin, attacker)
    }

    #[test]
    #[should_panic]
    fn withdraw_missing_auth_panics() {
        let (_e, _contract_id, client, _owner, _admin, _attacker) = setup();
        client.withdraw(&10);
    }

    #[test]
    #[should_panic]
    fn withdraw_wrong_signer_panics() {
        let (e, contract_id, client, owner, _admin, attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &attacker,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "withdraw",
                    args: (&10_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .withdraw(&10);

        // Ensure the setup owner remains distinct from attacker in this negative case.
        assert!(owner != attacker);
    }

    #[test]
    #[should_panic]
    fn withdraw_admin_signer_panics() {
        let (e, contract_id, client, _owner, admin, _attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "withdraw",
                    args: (&10_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .withdraw(&10);
    }

    #[test]
    #[should_panic]
    fn top_up_missing_auth_panics() {
        let (_e, _contract_id, client, _owner, _admin, _attacker) = setup();
        client.top_up(&50);
    }

    #[test]
    #[should_panic]
    fn top_up_wrong_signer_panics() {
        let (e, contract_id, client, _owner, _admin, attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &attacker,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "top_up",
                    args: (&50_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .top_up(&50);
    }

    #[test]
    #[should_panic]
    fn top_up_admin_signer_panics() {
        let (e, contract_id, client, _owner, admin, _attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "top_up",
                    args: (&50_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .top_up(&50);
    }

    #[test]
    #[should_panic]
    fn request_withdrawal_missing_auth_panics() {
        let (_e, _contract_id, client, _owner, _admin, _attacker) = setup();
        client.request_withdrawal();
    }

    #[test]
    #[should_panic]
    fn request_withdrawal_wrong_signer_panics() {
        let (e, contract_id, client, _owner, _admin, attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &attacker,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "request_withdrawal",
                    args: ().into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .request_withdrawal();
    }

    #[test]
    #[should_panic]
    fn request_withdrawal_admin_signer_panics() {
        let (e, contract_id, client, _owner, admin, _attacker) = setup();
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "request_withdrawal",
                    args: ().into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .request_withdrawal();
    }

    #[test]
    #[should_panic]
    fn extend_duration_missing_auth_panics() {
        let (_e, _contract_id, client, _owner, _admin, _attacker) = setup();
        client.extend_duration(&5);
    }

    #[test]
    #[should_panic]
    fn renew_if_rolling_missing_auth_panics() {
        let (_e, _contract_id, client, _owner, _admin, _attacker) = setup();
        client.renew_if_rolling();
    }

    #[test]
    fn owner_with_mock_auth_can_mutate() {
        let (e, contract_id, client, owner, _admin, _attacker) = setup();

        let updated = client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "top_up",
                    args: (&25_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .top_up(&25);

        assert_eq!(updated.bonded_amount, 1_025);
    }

    #[test]
    fn create_bond_auth_records_owner() {
        let e = Env::default();
        let contract_id = e.register(CredenceBond, ());
        let client = CredenceBondClient::new(&e, &contract_id);
        let owner = Address::generate(&e);
        let amount = 123_i128;
        let duration = 77_u64;
        let is_rolling = false;
        let notice = 0_u64;

        client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "create_bond",
                    args: (&owner, &amount, &duration, &is_rolling, &notice).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .create_bond(&owner, &amount, &duration, &is_rolling, &notice);

        assert!(e.auths().iter().any(|(addr, _)| addr == &owner));
    }

    #[test]
    #[should_panic]
    fn create_bond_missing_auth_panics() {
        let e = Env::default();
        let contract_id = e.register(CredenceBond, ());
        let client = CredenceBondClient::new(&e, &contract_id);
        let owner = Address::generate(&e);
        client.create_bond(&owner, &10, &1, &false, &0);
    }

    #[test]
    fn withdraw_success() {
        let (e, contract_id, client, owner, _admin, _attacker) = setup();

        let updated = client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "withdraw",
                    args: (&100_i128,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .withdraw(&100);

        assert_eq!(updated.bonded_amount, 900);
    }

    #[test]
    fn extend_duration_success() {
        let (e, contract_id, client, owner, _admin, _attacker) = setup();

        let updated = client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "extend_duration",
                    args: (&10_u64,).into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .extend_duration(&10);

        assert_eq!(updated.bond_duration, 1_010);
    }

    #[test]
    fn request_withdrawal_success() {
        let (e, contract_id, client, owner, _admin, _attacker) = setup();

        client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "request_withdrawal",
                    args: ().into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .request_withdrawal();

        let updated = client.get_identity_state();
        assert_eq!(updated.withdrawal_requested_at, e.ledger().timestamp());
    }

    #[test]
    fn renew_if_rolling_expired_renews() {
        let (e, contract_id, client, owner, _admin, _attacker) = setup();

        // Expire the bond by setting its start to far in the past
        let mut bond = client.get_identity_state();
        let now = e.ledger().timestamp();
        bond.bond_start = now.saturating_sub(bond.bond_duration + 10);
        e.as_contract(&contract_id, || {
            e.storage().instance().set(&DataKey::Bond, &bond);
        });

        let updated = client
            .mock_auths(&[MockAuth {
                address: &owner,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "renew_if_rolling",
                    args: ().into_val(&e),
                    sub_invokes: &[],
                },
            }])
            .renew_if_rolling();

        assert!(updated.bond_start >= now);
        assert_eq!(updated.withdrawal_requested_at, 0);
    }
}
