use crate::{
    contract, msg::TokenPriceInfo, AllPricesResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
    PendingUpdatesResponse, PriceHistoryResponse, QueryMsg,
    TokenPriceResponse, WhitelistedUpdatersResponse,
};
use cosmwasm_std::{Decimal, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};
use std::collections::HashMap;

fn gg_oracle_price_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        contract::execute,
        contract::instantiate,
        contract::query,
    ))
}

#[test]
fn instantiating_with_all_parameters_specified() {
    let mut app = App::default();

    let code_id = app.store_code(gg_oracle_price_contract());

    let sender = "sender".into_addr();
    let admin = "admin".into_addr();

    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender.clone(),
            &InstantiateMsg {
                owner: Some(sender.clone().to_string()),
                admin: Some(admin.clone().to_string()),
                price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                supported_tokens: vec!["pax-gold".parse().unwrap()],
                whitelisted_updaters: vec![admin.clone().to_string()],
            },
            &[],
            "GG Oracle Price Label",
            Some(admin.clone().to_string()),
        )
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(admin.to_string(), res.admin);
    assert_eq!(sender.to_string(), res.owner);
    assert_eq!(Decimal::from_ratio(5u128, 100u128), res.price_deviation_threshold);
}

mod instantiation_tests {
    use super::*;

    #[test]
    fn instantiate_with_default_owner() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());
        let sender = "sender".into_addr();
        let admin = "admin".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &InstantiateMsg {
                    owner: None, // Default to sender
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".parse().unwrap()],
                    whitelisted_updaters: vec![],
                },
                &[],
                "GG Oracle Price",
                None,
            )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(sender.to_string(), res.owner);
    }

    #[test]
    fn instantiate_with_default_admin() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());
        let sender = "sender".into_addr();
        let owner = "custom_owner".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &InstantiateMsg {
                    owner: Some(owner.to_string()),
                    admin: None, // Default to owner
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["eth".parse().unwrap()],
                    whitelisted_updaters: vec![],
                },
                &[],
                "GG Oracle Price",
                None,
            )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(owner.to_string(), res.admin);
        assert_eq!(owner.to_string(), res.owner);
    }

    #[test]
    fn validate_price_deviation_threshold() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());
        let sender = "sender".into_addr();

        // Test with valid threshold
        let contract_addr_valid = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &InstantiateMsg {
                    owner: None,
                    admin: None,
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec![],
                    whitelisted_updaters: vec![],
                },
                &[],
                "Valid Threshold",
                None,
            )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr_valid, &QueryMsg::Config {})
            .unwrap();
        assert_eq!(Decimal::from_ratio(5u128, 100u128), res.price_deviation_threshold);

        // Test with invalid threshold (0)
        let err = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &InstantiateMsg {
                    owner: None,
                    admin: None,
                    price_deviation_threshold: Decimal::zero(),
                    supported_tokens: vec![],
                    whitelisted_updaters: vec![],
                },
                &[],
                "Invalid Threshold Zero",
                None,
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Threshold must be between"));

        // Test with invalid threshold (> 100%)
        let err = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &InstantiateMsg {
                    owner: None,
                    admin: None,
                    price_deviation_threshold: Decimal::from_ratio(101u128, 100u128),
                    supported_tokens: vec![],
                    whitelisted_updaters: vec![],
                },
                &[],
                "Invalid Threshold High",
                None,
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Threshold must be between"));
    }
}

mod admin_operation_tests {
    use cosmwasm_std::Addr;
    use super::*;

    fn setup() -> (App, Addr, Addr, Addr, Addr) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let non_admin = "non_admin".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".parse().unwrap()],
                    whitelisted_updaters: vec![],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        (app, Addr::unchecked(contract_addr), owner, admin, non_admin)
    }

    #[test]
    fn owner_can_update_owner() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let new_owner = "new_owner".into_addr();

        // Owner updates owner
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateOwner { new_owner: new_owner.to_string() },
            &[],
        )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(new_owner.to_string(), res.owner);
    }

    #[test]
    fn non_owner_cannot_update_owner() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let new_owner = "new_owner".into_addr();

        // Admin tries to update owner
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateOwner { new_owner: new_owner.to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn owner_can_update_admin() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let new_admin = "new_admin".into_addr();

        // Owner updates admin
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateAdmin { new_admin: new_admin.to_string() },
            &[],
        )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(new_admin.to_string(), res.admin);
    }

    #[test]
    fn non_owner_cannot_update_admin() {
        let (mut app, contract_addr, _, admin, non_admin) = setup();
        let new_admin = "new_admin".into_addr();

        // Admin tries to update admin
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateAdmin { new_admin: new_admin.to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));

        // Non-admin tries to update admin
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateAdmin { new_admin: new_admin.to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn admin_can_set_deviation_threshold() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let new_threshold = Decimal::from_ratio(10u128, 100u128);

        // Admin updates threshold
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::SetDeviationThreshold { threshold: new_threshold },
            &[],
        )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(new_threshold, res.price_deviation_threshold);
    }

    #[test]
    fn owner_can_set_deviation_threshold() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let new_threshold = Decimal::from_ratio(15u128, 100u128);

        // Owner updates threshold
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::SetDeviationThreshold { threshold: new_threshold },
            &[],
        )
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(new_threshold, res.price_deviation_threshold);
    }

    #[test]
    fn non_admin_cannot_set_deviation_threshold() {
        let (mut app, contract_addr, _, _, non_admin) = setup();
        let new_threshold = Decimal::from_ratio(20u128, 100u128);

        // Non-admin tries to update threshold
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::SetDeviationThreshold { threshold: new_threshold },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn invalid_threshold_is_rejected() {
        let (mut app, contract_addr, _, admin, _) = setup();

        // Try to set zero threshold
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::SetDeviationThreshold { threshold: Decimal::zero() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("Threshold must be between"));

        // Try to set threshold > 100%
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::SetDeviationThreshold { threshold: Decimal::from_ratio(101u128, 100u128) },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Threshold must be between"));
    }
}

mod whitelist_management_tests {
    use super::*;
    use cosmwasm_std::Addr;

    fn setup() -> (App, Addr,Addr,Addr,Addr) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let non_admin = "non_admin".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".parse().unwrap()],
                    whitelisted_updaters: vec!["initial_updater".into_addr().to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        (app, Addr::unchecked(contract_addr), owner, admin, non_admin)
    }

    #[test]
    fn admin_can_add_whitelisted_updater() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let new_updater = "new_updater";

        // Admin adds updater
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddWhitelistedUpdater { updater: new_updater.into_addr().to_string() },
            &[],
        )
            .unwrap();

        let res: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert!(res.updaters.contains(&new_updater.into_addr().to_string()));
    }

    #[test]
    fn owner_can_add_whitelisted_updater() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let new_updater = "new_updater_from_owner";

        // Owner adds updater
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddWhitelistedUpdater { updater: new_updater.into_addr().to_string() },
            &[],
        )
            .unwrap();

        let res: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert!(res.updaters.contains(&new_updater.into_addr().to_string()));
    }

    #[test]
    fn non_admin_cannot_add_whitelisted_updater() {
        let (mut app, contract_addr, _, _, non_admin) = setup();
        let new_updater = "unauthorized_updater";

        // Non-admin tries to add updater
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddWhitelistedUpdater { updater: new_updater.to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn adding_already_whitelisted_updater_fails() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let existing_updater = "initial_updater";

        // Try to add already whitelisted updater
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddWhitelistedUpdater { updater: existing_updater.into_addr().to_string() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("already whitelisted"));
    }

    #[test]
    fn admin_can_remove_whitelisted_updater() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let existing_updater = "initial_updater";

        // Admin removes updater
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveWhitelistedUpdater { updater: existing_updater.into_addr().to_string() },
            &[],
        )
            .unwrap();

        let res: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert!(!res.updaters.contains(&existing_updater.to_string()));
    }

    #[test]
    fn owner_can_remove_whitelisted_updater() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let existing_updater = "initial_updater";

        // Owner removes updater
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveWhitelistedUpdater { updater: existing_updater.into_addr().to_string() },
            &[],
        )
            .unwrap();

        let res: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert!(!res.updaters.contains(&existing_updater.to_string()));
    }

    #[test]
    fn non_admin_cannot_remove_whitelisted_updater() {
        let (mut app, contract_addr, _, _, non_admin) = setup();
        let existing_updater = "initial_updater";


        // Non-admin tries to remove updater
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveWhitelistedUpdater { updater: existing_updater.to_string() },
            &[],
        )
            .unwrap_err();

        println!("{}",non_admin );
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn removing_non_whitelisted_updater_fails() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let non_existing_updater = "non_existing_updater";

        // Try to remove non-whitelisted updater
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveWhitelistedUpdater { updater: non_existing_updater.into_addr().to_string() },
            &[],
        )
            .unwrap_err();



        assert!(err.root_cause().to_string().contains("not whitelisted"));
    }
}

mod token_management_tests {
    use super::*;
    use crate::msg::SupportedTokensResponse;
    use cosmwasm_std::Addr;

    fn setup() -> (App, String, Addr, Addr, Addr) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let non_admin = "non_admin".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".parse().unwrap()],
                    whitelisted_updaters: vec![],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        (app, contract_addr, owner, admin, non_admin)
    }

    #[test]
    fn admin_can_add_supported_token() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let new_token = "eth";

        // Admin adds token
        app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::AddSupportedToken { token_id: new_token.to_string() },
            &[],
        )
            .unwrap();

        let res: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::SupportedTokens {})
            .unwrap();

        assert!(res.tokens.contains(&new_token.to_string()));
    }

    #[test]
    fn owner_can_add_supported_token() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let new_token = "sol";

        // Owner adds token
        app.execute_contract(
            owner.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::AddSupportedToken { token_id: new_token.to_string() },
            &[],
        )
            .unwrap();

        let res: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::SupportedTokens {})
            .unwrap();

        assert!(res.tokens.contains(&new_token.to_string()));
    }

    #[test]
    fn non_admin_cannot_add_supported_token() {
        let (mut app, contract_addr, _, _, non_admin) = setup();
        let new_token = "avax";

        // Non-admin tries to add token
        let err = app.execute_contract(
            non_admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::AddSupportedToken { token_id: new_token.to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn adding_already_supported_token_fails() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let existing_token = "btc";

        // Try to add already supported token
        let err = app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::AddSupportedToken { token_id: existing_token.to_string() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("Token btc already supported"));
    }

    #[test]
    fn admin_can_remove_supported_token() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let existing_token = "btc";

        // Admin removes token
        app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::RemoveSupportedToken { token_id: existing_token.to_string() },
            &[],
        )
            .unwrap();

        let res: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::SupportedTokens {})
            .unwrap();

        assert!(!res.tokens.contains(&existing_token.to_string()));
    }

    #[test]
    fn owner_can_remove_supported_token() {
        let (mut app, contract_addr, owner, _, _) = setup();
        let existing_token = "btc";

        // Owner removes token
        app.execute_contract(
            owner.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::RemoveSupportedToken { token_id: existing_token.to_string() },
            &[],
        )
            .unwrap();

        let res: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::SupportedTokens {})
            .unwrap();

        assert!(!res.tokens.contains(&existing_token.to_string()));
    }

    #[test]
    fn non_admin_cannot_remove_supported_token() {
        let (mut app, contract_addr, _, _, non_admin) = setup();
        let existing_token = "btc";

        // Non-admin tries to remove token
        let err = app.execute_contract(
            non_admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::RemoveSupportedToken { token_id: existing_token.to_string() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn removing_non_supported_token_fails() {
        let (mut app, contract_addr, _, admin, _) = setup();
        let non_existing_token = "not_a_token";

        // Try to remove non-supported token
        let err = app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::RemoveSupportedToken { token_id: non_existing_token.to_string() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("not supported"));
    }
}

mod price_update_tests {
    use cosmwasm_std::Addr;
    use super::*;
    use crate::msg::PriceData;

    fn setup() -> (App, String, String, String, String, String) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let updater = "updater".into_addr();
        let non_updater = "non_updater".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string(), "eth".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        // Set initial price for BTC
        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        (app, contract_addr, owner.to_string(), admin.to_string(), updater.to_string(), non_updater.to_string())
    }

    #[test]
    fn whitelisted_updater_can_update_single_price() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // Update price
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(41000u128, 1u128), res.price);
    }

    #[test]
    fn admin_can_update_single_price() {
        let (mut app, contract_addr, _, admin, _, _) = setup();

        // Admin updates price
        app.execute_contract(
            Addr::unchecked(admin.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(42000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(42000u128, 1u128), res.price);
    }

    #[test]
    fn owner_can_update_single_price() {
        let (mut app, contract_addr, owner, _, _, _) = setup();

        // Owner updates price
        app.execute_contract(
            Addr::unchecked(owner.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(42000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(42000u128, 1u128), res.price);
    }

    #[test]
    fn non_whitelisted_updater_cannot_update_price() {
        let (mut app, contract_addr, _, _, _, non_updater) = setup();

        // Non-whitelisted updater tries to update price
        let err = app.execute_contract(
            Addr::unchecked(non_updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(44000u128, 1u128) }
            },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn updating_non_supported_token_fails() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // Try to update non-supported token
        let err = app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "xrp".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(1u128, 1u128) }
            },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("Token xrp not supported"));
    }

    #[test]
    fn updating_prices_within_threshold_succeeds() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // Update price within 5% threshold (initial price is 40000)
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(41000u128, 1u128), res.price);
    }

    #[test]
    fn updating_prices_beyond_threshold_creates_pending_update() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // Update price beyond 5% threshold (initial price is 40000)
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(50000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Price should not update immediately
        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.price);

        // A pending update should be created
        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(1, res.updates.len());
        assert_eq!("btc", res.updates[0].token_id);
        assert_eq!(Decimal::from_ratio(50000u128, 1u128), res.updates[0].new_price);
    }

    #[test]
    fn batch_price_updates_for_multiple_tokens() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // Create price data for batch update
        let mut prices = HashMap::new();
        prices.insert("btc".to_string(), TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) });
        prices.insert("eth".to_string(), TokenPriceInfo { usd: Decimal::from_ratio(2000u128, 1u128) });

        // Execute batch update
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdatePrices {
                price_data: PriceData { prices }
            },
            &[],
        )
            .unwrap();

        // Check BTC price
        let res_btc: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(41000u128, 1u128), res_btc.price);

        // Check ETH price
        let res_eth: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "eth".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(2000u128, 1u128), res_eth.price);
    }

    #[test]
    fn batch_updates_with_some_prices_within_threshold_and_some_beyond() {
        let (mut app, contract_addr, _, _, updater, _) = setup();

        // First set ETH price
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "eth".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(2000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Create price data for batch update
        let mut prices = HashMap::new();
        // Within threshold for BTC
        prices.insert("btc".to_string(), TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) });
        // Beyond threshold for ETH
        prices.insert("eth".to_string(), TokenPriceInfo { usd: Decimal::from_ratio(3000u128, 1u128) });

        // Execute batch update
        app.execute_contract(
            Addr::unchecked(updater.clone()),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdatePrices {
                price_data: PriceData { prices }
            },
            &[],
        )
            .unwrap();

        // Check BTC price (should update)
        let res_btc: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(41000u128, 1u128), res_btc.price);

        // Check ETH price (should not update)
        let res_eth: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "eth".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(2000u128, 1u128), res_eth.price);

        // Check pending updates
        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(1, res.updates.len());
        assert_eq!("eth", res.updates[0].token_id);
        assert_eq!(Decimal::from_ratio(3000u128, 1u128), res.updates[0].new_price);
    }
}

mod pending_update_tests {
    use cosmwasm_std::Addr;
    use super::*;

    fn setup_with_pending_update() -> (App, Addr, Addr, Addr, Addr) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let updater = "updater".into_addr();
        let non_admin = "non_admin".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        // Set initial price for BTC
        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Create a pending update by setting price beyond threshold
        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(50000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        (app, Addr::unchecked(contract_addr), owner, admin, non_admin)
    }

    #[test]
    fn admin_can_approve_pending_price() {
        let (mut app, contract_addr, _, admin, _) = setup_with_pending_update();

        // Admin approves price
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::ApprovePrice {
                token_id: "btc".to_string(),
                price: Decimal::from_ratio(50000u128, 1u128)
            },
            &[],
        )
            .unwrap();

        // Check price was updated
        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(50000u128, 1u128), res.price);

        // Check pending update was removed
        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(0, res.updates.len());
    }

    #[test]
    fn owner_can_approve_pending_price() {
        let (mut app, contract_addr, owner, _, _) = setup_with_pending_update();

        // Owner approves price
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::ApprovePrice {
                token_id: "btc".to_string(),
                price: Decimal::from_ratio(50000u128, 1u128)
            },
            &[],
        )
            .unwrap();

        // Check price was updated
        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(50000u128, 1u128), res.price);
    }

    #[test]
    fn non_admin_cannot_approve_pending_price() {
        let (mut app, contract_addr, _, _, non_admin) = setup_with_pending_update();

        // Non-admin tries to approve price
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::ApprovePrice {
                token_id: "btc".to_string(),
                price: Decimal::from_ratio(50000u128, 1u128)
            },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn approving_non_existent_pending_update_fails() {
        let (mut app, contract_addr, _, admin, _) = setup_with_pending_update();

        // Try to approve update for non-existent token
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::ApprovePrice {
                token_id: "eth".to_string(),
                price: Decimal::from_ratio(2000u128, 1u128)
            },
            &[],
        )
            .unwrap_err();



        assert!(err.root_cause().to_string().contains("Token eth not supported"));
    }

    #[test]
    fn admin_can_reject_pending_price() {
        let (mut app, contract_addr, _, admin, _) = setup_with_pending_update();

        // Admin rejects price
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RejectPrice { token_id: "btc".to_string() },
            &[],
        )
            .unwrap();

        // Check original price is maintained
        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.price);

        // Check pending update was removed
        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(0, res.updates.len());
    }

    #[test]
    fn owner_can_reject_pending_price() {
        let (mut app, contract_addr, owner, _, _) = setup_with_pending_update();

        // Owner rejects price
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RejectPrice { token_id: "btc".to_string() },
            &[],
        )
            .unwrap();

        // Check pending update was removed
        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(0, res.updates.len());
    }

    #[test]
    fn non_admin_cannot_reject_pending_price() {
        let (mut app, contract_addr, _, _, non_admin) = setup_with_pending_update();

        // Non-admin tries to reject price
        let err = app.execute_contract(
            non_admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RejectPrice { token_id: "btc".to_string() },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn rejecting_non_existent_pending_update_fails() {
        let (mut app, contract_addr, _, admin, _) = setup_with_pending_update();

        // Try to reject update for non-existent token
        let err = app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RejectPrice { token_id: "eth".to_string() },
            &[],
        )
            .unwrap_err();


        assert!(err.root_cause().to_string().contains("No pending update"));
    }
}

mod query_tests {
    use cosmwasm_std::Addr;
    use super::*;
    use crate::msg::SupportedTokensResponse;
    use thiserror::__private::AsDynError;

    fn setup_with_prices() -> (App, String) {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let updater = "updater".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string(), "eth".to_string(), "sol".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap()
            .to_string();

        // Set prices
        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "eth".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(2000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Create pending update
        app.execute_contract(
            updater.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(50000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        (app, contract_addr)
    }

    #[test]
    fn query_config_returns_correct_values() {
        let (app, contract_addr) = setup_with_prices();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!("owner".into_addr().to_string(), res.owner);
        assert_eq!("admin".into_addr().to_string(), res.admin);
        assert_eq!(Decimal::from_ratio(5u128, 100u128), res.price_deviation_threshold);
    }

    #[test]
    fn query_token_price_for_existing_price() {
        let (app, contract_addr) = setup_with_prices();

        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!("btc", res.token_id);
        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.price);
    }

    #[test]
    fn query_token_price_for_non_existent_price_fails() {
        let (app, contract_addr) = setup_with_prices();

        let err = app
            .wrap()
            .query_wasm_smart::<TokenPriceResponse>(contract_addr, &QueryMsg::TokenPrice { token_id: "sol".to_string() })
            .unwrap_err();

        assert!(err.as_dyn_error().to_string().contains("No price data for token"));
    }

    #[test]
    fn query_token_price_for_non_supported_token_fails() {
        let (app, contract_addr) = setup_with_prices();

        let err = app
            .wrap()
            .query_wasm_smart::<TokenPriceResponse>(contract_addr, &QueryMsg::TokenPrice { token_id: "xrp".to_string() })
            .unwrap_err();

        assert!(err.to_string().contains("Token xrp not supported"));
    }

    #[test]
    fn query_all_prices_returns_correct_list() {
        let (app, contract_addr) = setup_with_prices();

        let res: AllPricesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::AllPrices {})
            .unwrap();

        assert_eq!(2, res.prices.len());
        assert!(res.prices.contains_key("btc"));
        assert!(res.prices.contains_key("eth"));
        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.prices.get("btc").unwrap().usd);
        assert_eq!(Decimal::from_ratio(2000u128, 1u128), res.prices.get("eth").unwrap().usd);
    }

    #[test]
    fn query_price_history_with_default_parameters() {
        let (app, contract_addr) = setup_with_prices();

        let res: PriceHistoryResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PriceHistory {
                token_id: "btc".to_string(),
                start_time: None,
                end_time: None,
                limit: None,
            })
            .unwrap();

        assert_eq!("btc", res.token_id);
        println!("History: {:?}", res.history);

        assert_eq!(res.history.len(), 1);
        assert_eq!(res.history[0].price, Decimal::from_ratio(40000u128, 1u128));
    }

    #[test]
    fn query_pending_updates_returns_correct_list() {
        let (app, contract_addr) = setup_with_prices();

        let res: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        println!("{:?}",res.updates);
        assert_eq!(1, res.updates.len());
        assert_eq!("btc", res.updates[0].token_id);
        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.updates[0].current_price);
        assert_eq!(Decimal::from_ratio(50000u128, 1u128), res.updates[0].new_price);
    }

    #[test]
    fn query_supported_tokens_returns_only_supported_tokens() {
        let (mut app, contract_addr) = setup_with_prices();
        let admin = "admin".into_addr();

        // Remove support for one token
        app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::RemoveSupportedToken { token_id: "sol".to_string() },
            &[],
        )
            .unwrap();

        let res: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::SupportedTokens {})
            .unwrap();

        assert_eq!(2, res.tokens.len());
        assert!(res.tokens.contains(&"btc".to_string()));
        assert!(res.tokens.contains(&"eth".to_string()));
        assert!(!res.tokens.contains(&"sol".to_string()));
    }

    #[test]
    fn query_whitelisted_updaters_returns_correct_list() {
        let (mut app, contract_addr) = setup_with_prices();
        let admin = "admin".into_addr();

        // Add another updater
        app.execute_contract(
            admin.clone(),
            Addr::unchecked(contract_addr.clone()),
            &ExecuteMsg::AddWhitelistedUpdater { updater: "new_updater".into_addr().to_string() },
            &[],
        )
            .unwrap();

        let res: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert_eq!(2, res.updaters.len());
        assert!(res.updaters.contains(&"updater".into_addr().to_string()));
        assert!(res.updaters.contains(&"new_updater".into_addr().to_string()));
    }
}

mod integration_scenario_tests {
    use super::*;
    use crate::msg::SupportedTokensResponse;

    #[test]
    fn test_full_price_update_lifecycle() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let admin = "admin".into_addr();
        let updater = "updater".into_addr();

        // Instantiate contract
        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: Some(admin.clone().to_string()),
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap();

        // Step 1: Initial price update
        app.execute_contract(
            updater.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Verify initial price
        let res: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(40000u128, 1u128), res.price);

        // Step 2: Update beyond threshold creates pending update
        app.execute_contract(
            updater.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(50000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Verify pending update created and price unchanged
        let pending: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(1, pending.updates.len());
        assert_eq!("btc", pending.updates[0].token_id);
        assert_eq!(Decimal::from_ratio(50000u128, 1u128), pending.updates[0].new_price);

        let price: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(40000u128, 1u128), price.price);

        // Step 3: Admin approves the pending update
        app.execute_contract(
            admin.clone(),
            contract_addr.clone(),
            &ExecuteMsg::ApprovePrice {
                token_id: "btc".to_string(),
                price: Decimal::from_ratio(50000u128, 1u128)
            },
            &[],
        )
            .unwrap();

        // Verify price updated and pending updates cleared
        let price: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(50000u128, 1u128), price.price);

        let pending: PendingUpdatesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::PendingUpdates {})
            .unwrap();

        assert_eq!(0, pending.updates.len());
    }

    #[test]
    fn test_price_history_accumulation_over_time() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let updater = "updater".into_addr();

        // Instantiate contract
        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: None,
                    price_deviation_threshold: Decimal::from_ratio(10u128, 100u128), // Higher threshold to avoid pending updates
                    supported_tokens: vec!["eth".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap();

        // Update price multiple times
        let prices = vec![
            Decimal::from_ratio(2000u128, 1u128),
            Decimal::from_ratio(2100u128, 1u128),
            Decimal::from_ratio(2050u128, 1u128),
            Decimal::from_ratio(2200u128, 1u128),
            Decimal::from_ratio(2150u128, 1u128),
        ];

        for price in prices.iter() {
            app.execute_contract(
                updater.clone(),
                contract_addr.clone(),
                &ExecuteMsg::UpdateSinglePrice {
                    token_id: "eth".to_string(),
                    price_info: TokenPriceInfo { usd: *price }
                },
                &[],
            )
                .unwrap();

            // Move time forward
            app.update_block(|block| {
                block.time = block.time.plus_seconds(300);
            });
        }

        // Query price history
        let history: PriceHistoryResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::PriceHistory {
                token_id: "eth".to_string(),
                start_time: None,
                end_time: None,
                limit: None
            })
            .unwrap();

        // Verify history length
        assert_eq!(5, history.history.len());

        // Verify prices in reverse order (most recent first)
        assert_eq!(Decimal::from_ratio(2150u128, 1u128), history.history[0].price);
        assert_eq!(Decimal::from_ratio(2200u128, 1u128), history.history[1].price);
        assert_eq!(Decimal::from_ratio(2050u128, 1u128), history.history[2].price);
        assert_eq!(Decimal::from_ratio(2100u128, 1u128), history.history[3].price);
        assert_eq!(Decimal::from_ratio(2000u128, 1u128), history.history[4].price);

        // Test with limit
        let limited_history: PriceHistoryResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::PriceHistory {
                token_id: "eth".to_string(),
                start_time: None,
                end_time: None,
                limit: Some(3)
            })
            .unwrap();

        assert_eq!(3, limited_history.history.len());
        assert_eq!(Decimal::from_ratio(2150u128, 1u128), limited_history.history[0].price);
    }

    #[test]
    fn test_adding_removing_tokens_and_their_effect_on_queries() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let updater = "updater".into_addr();

        // Instantiate contract with one token
        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: None,
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string()],
                    whitelisted_updaters: vec![updater.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap();

        // Set initial price
        app.execute_contract(
            updater.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Add a new token
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddSupportedToken { token_id: "eth".to_string() },
            &[],
        )
            .unwrap();

        // Verify supported tokens list
        let tokens: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::SupportedTokens {})
            .unwrap();

        assert_eq!(2, tokens.tokens.len());
        assert!(tokens.tokens.contains(&"btc".to_string()));
        assert!(tokens.tokens.contains(&"eth".to_string()));

        // Update new token price
        app.execute_contract(
            updater.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "eth".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(2000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Check all prices
        let prices: AllPricesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::AllPrices {})
            .unwrap();

        assert_eq!(2, prices.prices.len());
        assert!(prices.prices.contains_key("btc"));
        assert!(prices.prices.contains_key("eth"));

        // Remove a token
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveSupportedToken { token_id: "btc".to_string() },
            &[],
        )
            .unwrap();

        // Verify supported tokens list
        let tokens: SupportedTokensResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::SupportedTokens {})
            .unwrap();

        assert_eq!(1, tokens.tokens.len());
        assert!(!tokens.tokens.contains(&"btc".to_string()));
        assert!(tokens.tokens.contains(&"eth".to_string()));

        // Check all prices
        let prices: AllPricesResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::AllPrices {})
            .unwrap();

        println!("{:?}",prices);

        // BTC price might still exist in storage but should be inaccessible via token query
        let err = app
            .wrap()
            .query_wasm_smart::<TokenPriceResponse>(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap_err();

        assert!(err.to_string().contains("Token btc not supported"));
    }

    #[test]
    fn test_adding_removing_updaters_and_their_effect_on_price_updates() {
        let mut app = App::default();
        let code_id = app.store_code(gg_oracle_price_contract());

        let owner = "owner".into_addr();
        let updater1 = "updater1".into_addr();
        let updater2 = "updater2".into_addr();

        // Instantiate contract with one updater
        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().to_string()),
                    admin: None,
                    price_deviation_threshold: Decimal::from_ratio(5u128, 100u128),
                    supported_tokens: vec!["btc".to_string()],
                    whitelisted_updaters: vec![updater1.to_string()],
                },
                &[],
                "Oracle Contract",
                None,
            )
            .unwrap();

        // Set initial price with updater1
        app.execute_contract(
            updater1.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(40000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Updater2 (not whitelisted) tries to update price and fails
        let err = app.execute_contract(
            updater2.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) }
            },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));

        // Add updater2 to whitelist
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::AddWhitelistedUpdater { updater: updater2.to_string() },
            &[],
        )
            .unwrap();

        // Verify whitelisted updaters
        let updaters: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert_eq!(2, updaters.updaters.len());
        assert!(updaters.updaters.contains(&updater1.to_string()));
        assert!(updaters.updaters.contains(&updater2.to_string()));

        // Now updater2 can update price
        app.execute_contract(
            updater2.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(41000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Verify price was updated
        let price: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(41000u128, 1u128), price.price);

        // Remove updater1 from whitelist
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::RemoveWhitelistedUpdater { updater: updater1.to_string() },
            &[],
        )
            .unwrap();

        // Verify whitelisted updaters
        let updaters: WhitelistedUpdatersResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::WhitelistedUpdaters {})
            .unwrap();

        assert_eq!(1, updaters.updaters.len());
        assert!(!updaters.updaters.contains(&updater1.to_string()));
        assert!(updaters.updaters.contains(&updater2.to_string()));

        // Updater1 tries to update price and fails
        let err = app.execute_contract(
            updater1.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(42000u128, 1u128) }
            },
            &[],
        )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));

        // Owner can still update prices (implicit admin/owner privileges)
        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecuteMsg::UpdateSinglePrice {
                token_id: "btc".to_string(),
                price_info: TokenPriceInfo { usd: Decimal::from_ratio(42000u128, 1u128) }
            },
            &[],
        )
            .unwrap();

        // Verify price was updated
        let price: TokenPriceResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenPrice { token_id: "btc".to_string() })
            .unwrap();

        assert_eq!(Decimal::from_ratio(42000u128, 1u128), price.price);
    }
}

