use radix_engine::ledger::*;
use radix_engine::transaction::*;
use scrypto::prelude::*;


#[test]
fn try_withdraw_without_added_recipients_must_be_failed() {
    // Set up environment.

    let mut ledger = InMemoryLedger::with_bootstrap();
    let mut test_env = TestEnv::new(&mut ledger, RADIX_TOKEN);
    assert_eq!(test_env.get_balance(test_env.admin_account, RADIX_TOKEN).unwrap(), Decimal::from(1000000));

    // withdraw_token    
    let  (not_recipient_key, not_recipient_address)  = test_env.new_account();
    let not_withdraw_receipt =  test_env.withdraw_token(not_recipient_key, not_recipient_address);
    assert!(!not_withdraw_receipt.success);
    let log_message = &not_withdraw_receipt.logs.get(0).unwrap().1;
    assert!(log_message.starts_with("Panicked at 'Insufficient balance'"));
}

#[test]
fn try_withdraw_already_done_must_be_failed() {
    // Set up environment.

    let mut ledger = InMemoryLedger::with_bootstrap();
    let mut test_env = TestEnv::new(&mut ledger, RADIX_TOKEN);
    assert_eq!(test_env.get_balance(test_env.admin_account, RADIX_TOKEN).unwrap(), Decimal::from(1000000));
    let  (not_recipient_key, not_recipient_address)  = test_env.new_account();
    let token_by_recipient : Decimal =  Decimal::from(100); 
    // addRecipient
    test_env.add_recipient(not_recipient_address, RADIX_TOKEN ,token_by_recipient );

    // withdraw_token    
    let withdraw_receipt =  test_env.withdraw_token(not_recipient_key, not_recipient_address);
    assert!(withdraw_receipt.success);

    let already_withdraw_receipt =  test_env.withdraw_token(not_recipient_key, not_recipient_address);
    assert!(!already_withdraw_receipt.success);
    
    let log_message = &already_withdraw_receipt.logs.get(0).unwrap().1;
    assert!(log_message.starts_with("Panicked at 'withdraw already done'"));
}

#[test]
fn try_withdraw_after_added_recipients_must_be_succeeded() {
    // Set up environment.

    let mut ledger = InMemoryLedger::with_bootstrap();
    let token_by_recipient : Decimal =  Decimal::from(100); 
    let mut test_env = TestEnv::new(&mut ledger, RADIX_TOKEN);
    assert_eq!(test_env.get_balance(test_env.admin_account, RADIX_TOKEN).unwrap(),Decimal::from(1000000));

    // AddRecipients
    let recipient_count  = 2; 
    let mut recipients_accounts : Vec<(Address,Address)> =  Vec::new();

    for _ in 0..recipient_count {
        let (recipient_key, recipient_address)  = test_env.new_account();
        let add_recipient_receipt = test_env.add_recipient(recipient_address, RADIX_TOKEN ,token_by_recipient );
        assert!(add_recipient_receipt.success); 
        recipients_accounts.push((recipient_key, recipient_address));  
    }

    assert_eq!(test_env.get_balance(test_env.admin_account, RADIX_TOKEN).unwrap() , Decimal::from(1000000) - token_by_recipient * recipient_count);

    
    for (recipient_key, recipient_address)  in recipients_accounts {
           // available token
            let available_token_receipt = test_env.available_token(recipient_key, recipient_address); 
            assert!(available_token_receipt.success); 
            assert_eq!(format!("available : {}", token_by_recipient), available_token_receipt.logs.get(0).unwrap().1);
            
            // withdraw_token
            let withdraw_receipt =  test_env.withdraw_token(recipient_key, recipient_address);
            assert!(withdraw_receipt.success); 
            assert_eq!(format!("withdraw_token : {}", token_by_recipient), withdraw_receipt.logs.get(0).unwrap().1);
            assert_eq!(test_env.get_balance(recipient_address, RADIX_TOKEN).unwrap(), Decimal::from(1000000) + token_by_recipient);
    }
}

struct TestEnv<'a> {
    executor: TransactionExecutor<'a, InMemoryLedger>,
    admin_key: Address,
    admin_account: Address,
    component: Address,
    admin_badge: Address,
    recipient_badge : Address
}

impl<'a> TestEnv<'a> {
    pub fn new(ledger: &'a mut InMemoryLedger, token_type : Address) -> Self {
        let mut executor = TransactionExecutor::new(ledger, 0, 0);

        let package = executor.publish_package(include_code!("airdrop_with_withdraw"));
        let admin_key = executor.new_public_key();
        let admin_account = executor.new_account(admin_key);

        let tx = TransactionBuilder::new(&executor)
            .call_function(package, "AirdropWithWithdraw", "new", vec!
            [
                token_type.to_string()
            ], Some(admin_account))
            .deposit_all_buckets(admin_account)
            .drop_all_bucket_refs()
            .build(vec![admin_key])
            .unwrap();
        let receipt = executor.run(tx, false).unwrap();
        println!("{:?}\n", receipt);
        assert!(receipt.success);

        let admin_badge = receipt.resource_def(0).unwrap();
        let recipient_badge = receipt.resource_def(2).unwrap(); 

        Self {
            executor,
            admin_key,
            admin_account,
            component: receipt.component(0).unwrap(),
            admin_badge,
            recipient_badge
        }
    }

    pub fn new_account(&mut self) -> (Address, Address) {
        let key = self.executor.new_public_key();
        return (key, self.executor.new_account(key))
    }

    

    pub fn add_recipient(&mut self, recipient: Address, token_address :Address ,tokens : Decimal) -> Receipt {
        let tx = TransactionBuilder::new(&self.executor)
            .call_method(
                self.component,
                "add_recipient",
                vec![
                    recipient.to_string(),
                    format!("{},{}", tokens, token_address),
                    format!("1,{}", self.admin_badge)
                ],
                Some(self.admin_account),
            )
            .drop_all_bucket_refs()
            .deposit_all_buckets(self.admin_account)
            .build(vec![self.admin_key])
            .unwrap();
        let receipt = self.executor.run(tx, false).unwrap();
        println!("{:?}\n", receipt);
        return receipt;
    }

    fn withdraw_token(&mut self,recipient_key : Address, recipient_address : Address) -> Receipt {
        let tx = TransactionBuilder::new(&self.executor)
            .call_method(
                self.component,
                "withdraw_token",
                vec![
                    format!("1,{}", self.recipient_badge)
                ],
                Some(recipient_address),
            )
            .drop_all_bucket_refs()
            .deposit_all_buckets(recipient_address)
            .build(vec![recipient_key])
            .unwrap();
        let receipt = self.executor.run(tx, false).unwrap();
        println!("{:?}\n", receipt);
        return receipt; 
        
    }

    fn available_token(&mut self,recipient_key : Address, recipient_address : Address) -> Receipt {
        let tx = TransactionBuilder::new(&self.executor)
            .call_method(
                self.component,
                "available_token",
                vec![
                    format!("1,{}", self.recipient_badge)
                ],
                Some(recipient_address),
            )
            .drop_all_bucket_refs()
            .deposit_all_buckets(recipient_address)
            .build(vec![recipient_key])
            .unwrap();
        let receipt = self.executor.run(tx, false).unwrap();
        println!("{:?}\n", receipt);
        return receipt; 
        
    }

    fn get_balance(&self, account: Address, token: Address) -> Result<Decimal, String> {
        let ledger = self.executor.ledger();
        let account_component = ledger.get_component(account).unwrap();
        let mut vaults = vec![];
        let _res = radix_engine::utils::format_data_with_ledger(
            account_component
                .state(radix_engine::model::Actor::SuperUser)
                .unwrap(),
            ledger,
            &mut vaults,
        ).unwrap();

        for vid in vaults {
            let vault = self.executor.ledger().get_vault(vid).unwrap();
            let amount = vault.amount(radix_engine::model::Actor::SuperUser).unwrap();
            let resource_def_address = vault
                .resource_address(radix_engine::model::Actor::SuperUser)
                .unwrap();
            if token == resource_def_address {
                return Ok(amount);
            }
        }

        return Err(format!(
            "No vault found for token {} in account {}",
            token, account
        ));
    }
}