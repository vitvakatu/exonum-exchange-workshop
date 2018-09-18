use exonum::api;
use exonum::api::ServiceApiBuilder;
use exonum::api::ServiceApiState;
use exonum::blockchain::{ExecutionResult, Service, Transaction, TransactionSet};
use exonum::crypto::{Hash, PublicKey};
use exonum::encoding;
use exonum::helpers::fabric::Context;
use exonum::helpers::fabric::ServiceFactory;
use exonum::messages::{Message, RawTransaction};
use exonum::node::TransactionSend;
use exonum::storage::{Fork, MapIndex, Snapshot};
use protocol::*;
use std::collections::BTreeMap;

// // // // // // // // // // CONSTANTS // // // // // // // // // //

const USD_BALANCE: u32 = 1_000_000;
const TOKEN_BALANCE: u32 = 1_000;

// // // // // // // // // // PERSISTENT DATA // // // // // // // // // //

encoding_struct! {
    struct Account {
        owner: &PublicKey,
        usd_balance: u32, // .00
        token_balance: u32, // .000000
        orders: Vec<u32>,
    }
}

impl Account {
    fn buy_tokens(&self, price: u32, amount: i32, id: u32) -> Self {
        let usd_balance = self.usd_balance() - (price as i32 * amount) as u32;
        let token_balance = self.token_balance() + amount as u32;
        let mut orders = self.orders();
        orders.push(id);
        Self::new(self.owner(), usd_balance, token_balance, orders)
    }

    fn sell_tokens(&self, price: u32, amount: i32, id: u32) -> Self {
        let usd_balance = self.usd_balance() + (price as i32 * amount) as u32;
        let token_balance = self.token_balance() - amount as u32;
        let mut orders = self.orders();
        orders.push(id);
        Self::new(self.owner(), usd_balance, token_balance, orders)
    }

    fn add_order_id(&self, id: u32) -> Self {
        let mut orders = self.orders();
        orders.push(id);
        Self::new(
            self.owner(),
            self.usd_balance(),
            self.token_balance(),
            orders,
        )
    }

    fn remove_order_by_id(&self, id: u32) -> Option<Self> {
        let mut orders = self.orders();
        if let Some(index) = orders.iter().position(|x| *x == id) {
            orders.remove(index);
            let res = Self::new(
                self.owner(),
                self.usd_balance(),
                self.token_balance(),
                orders,
            );
            Some(res)
        } else {
            None
        }
    }
}

encoding_struct! {
    struct Order {
        owner: &PublicKey,
        price: u32,
        amount: i32,
        id: u32,
    }
}

// // // // // // // // // // DATA LAYOUT // // // // // // // // // //

pub struct ExchangeSchema<T> {
    view: T,
}

impl<T: AsRef<Snapshot>> ExchangeSchema<T> {
    pub fn new(view: T) -> Self {
        ExchangeSchema { view }
    }

    pub fn accounts(&self) -> MapIndex<&Snapshot, PublicKey, Account> {
        MapIndex::new("cryptoexchange.accounts", self.view.as_ref())
    }

    pub fn account(&self, owner: &PublicKey) -> Option<Account> {
        self.accounts().get(owner)
    }

    pub fn orders(&self) -> MapIndex<&Snapshot, u32, Order> {
        MapIndex::new("cryptoexchange.orders", self.view.as_ref())
    }
}

impl<'a> ExchangeSchema<&'a mut Fork> {
    pub fn accounts_mut(&mut self) -> MapIndex<&mut Fork, PublicKey, Account> {
        MapIndex::new("cryptoexchange.accounts", &mut self.view)
    }

    pub fn orders_mut(&mut self) -> MapIndex<&mut Fork, u32, Order> {
        MapIndex::new("cryptoexchange.orders", &mut self.view)
    }
}

// // // // // // // // // // CONTRACTS // // // // // // // // // //

impl Transaction for TxCreate {
    fn verify(&self) -> bool {
        self.verify_signature(self.owner())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        trace!("TxOrder");
        let mut schema = ExchangeSchema::new(view);
        if schema.account(self.owner()).is_none() {
            let account = Account::new(self.owner(), USD_BALANCE, TOKEN_BALANCE, Vec::new());
            println!("Create the account: {:?}", account);
            schema.accounts_mut().put(self.owner(), account);
        }
        Ok(())
    }
}

impl Transaction for TxOrder {
    fn verify(&self) -> bool {
        self.verify_signature(self.owner())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let mut schema = ExchangeSchema::new(view);
        let account = schema.account(self.owner());
        if let Some(account) = account {
            let not_exists = !schema.orders_mut().contains(&self.id());
            if not_exists {
                let order = Order::new(self.owner(), self.price(), self.amount(), self.id());
                println!("Put the order <{}>: {:?}", self.id(), order);
                let account = {
                    if order.amount() > 0 {
                        account.buy_tokens(order.price(), order.amount(), order.id())
                    } else {
                        account.sell_tokens(order.price(), -order.amount(), order.id())
                    }
                };
                schema.accounts_mut().put(self.owner(), account);
                schema.orders_mut().put(&self.id(), order);
            }
        }
        Ok(())
    }
}

impl Transaction for TxCancel {
    fn verify(&self) -> bool {
        self.verify_signature(self.owner())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        trace!("TxCancel");
        let mut schema = ExchangeSchema::new(view);
        let account = schema.account(self.owner());
        if let Some(account) = account {
            let exists = schema.orders_mut().contains(&self.id());
            if exists {
                if let Some(account) = account.remove_order_by_id(self.id()) {
                    schema.orders_mut().remove(&self.id());
                    schema.accounts_mut().put(self.owner(), account);
                }
            }
        }
        Ok(())
    }
}

// // // // // // // // // // REST API // // // // // // // // // //

#[derive(Clone)]
struct ExchangeServiceApi;

#[derive(Debug, Serialize, Deserialize)]
struct AccountQuery {
    pub key: PublicKey,
}

#[derive(Debug, Serialize, Deserialize)]
struct OrdersResponse(pub BTreeMap<u32, Order>);

/// Response to an incoming transaction returned by the REST API.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Hash of the transaction.
    pub tx_hash: Hash,
}

impl ExchangeServiceApi {
    /// Endpoint for handling cryptocurrency transactions.
    pub fn post_transaction(
        state: &ServiceApiState,
        query: Transactions,
    ) -> api::Result<TransactionResponse> {
        let transaction: Box<dyn Transaction> = query.into();
        let tx_hash = transaction.hash();
        state.sender().send(transaction)?;
        Ok(TransactionResponse { tx_hash })
    }

    pub fn get_accout(state: &ServiceApiState, query: AccountQuery) -> api::Result<Account> {
        let public_key = query.key;

        let account = {
            let snapshot = state.snapshot();
            let schema = ExchangeSchema::new(snapshot);
            schema.account(&public_key)
        };

        if let Some(account) = account {
            Ok(account)
        } else {
            Err(api::Error::NotFound("Account not found".to_owned()))
        }
    }

    pub fn get_orders(state: &ServiceApiState, _query: ()) -> api::Result<OrdersResponse> {
        let snapshot = state.snapshot();
        let schema = ExchangeSchema::new(snapshot);
        let orders = schema.orders();
        let orders = orders.iter().collect::<BTreeMap<u32, Order>>();
        Ok(OrdersResponse(orders))
    }

    pub fn wire(builder: &mut ServiceApiBuilder) {
        builder
            .public_scope()
            .endpoint("v1/account", Self::get_accout)
            .endpoint("v1/orders", Self::get_orders)
            .endpoint_mut("v1/transaction", Self::post_transaction);
    }
}

// // // // // // // // // // SERVICE DECLARATION // // // // // // // // // //
pub struct ExchangeService;

impl Service for ExchangeService {
    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    fn service_name(&self) -> &'static str {
        "cryptoexchange"
    }

    fn state_hash(&self, _: &Snapshot) -> Vec<Hash> {
        vec![]
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<Transaction>, encoding::Error> {
        let tx = Transactions::tx_from_raw(raw)?;
        Ok(tx.into())
    }

    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
        ExchangeServiceApi::wire(builder);
    }
}

impl ServiceFactory for ExchangeService {
    fn service_name(&self) -> &str {
        "cryptoexchange"
    }

    fn make_service(&mut self, _run_context: &Context) -> Box<Service> {
        Box::new(ExchangeService)
    }
}
