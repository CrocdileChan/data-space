#[cfg(feature = "hyper")]
extern crate hyper;

use ipfs_api::IpfsClient;
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::{As, Hash};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::Currency};
use system::ensure_signed;
use std::io::Cursor;
use futures::future::Future;

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DataSpace<Hash,Balance> {
    user_id: Hash,
    user_name: Vec<u8>,
    // hashmap key is ipfs_hash
    data_to_price: Vec<Data<Hash,Balance>>,
    bought_list: Vec<Hash>,
    sold_list: Vec<Hash>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Data<Hash,Balance>{
    id: Hash,
    name: String,
    owner: Hash,
    ipfs_key: String,
    content: Vec<u8>,
    price:Balance,
}

decl_event! {
    pub enum Event<T>
    where
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as system::Trait>::Hash
     {
        Registered(AccountId),
        PriceSet(AccountId,Hash,Balance),
        Bought(AccountId,AccountId,Hash,Balance),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as DataStore {
        Owners get(owner_of): map T::AccountId => DataSpace<T::Hash,T::Balance>;

        // data id => data struct
        Data_info get(data): map T::Hash => Data<T::Hash,T::Balance>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn register_account(origin,user_name: Vec<u8>) -> Result {
            <Owners<T>>::insert(origin,DataSpace{
                user_id: origin,
                user_name: user_name,
                data_to_price: Vec::new(),
                bought_list: Vec::new(),
                sold_list: Vec::new(),
            });
            Self::deposit_event(RawEvent::Registered(origin));
            Ok(())
        }

        fn set_price(origin,data: Vec<u8>,price: T::Balance) -> Result{

            //Self::deposit_event(RawEvent::PriceSet(sender,data_id,new_price));
            Ok(())
        }

        fn buy_data(origin, seller: T::AccountId, data_id: T::Hash, pay_price: T::Balance) -> Result {
            let buyer = ensure_signed(origin)?;
            ensure!(buyer != seller, "you can't buy your own data");

            let data_info = Self::data(data_id).ok_or("no data for sell")?;
            let data_price = data_info.price;
            ensure!(data_price <= pay_price, "the data you want to buy costs more than your pay");

            <balances::Module<T> as Currency<_>>::transfer(&buyer, &seller, data_price)?;

            let mut seller_ds = Self::owner_of(&seller).ok_or("no seller account you want")?;
            let mut buyer_ds = Self::owner_of(&buyer).ok_or("no account registered to buy data")?;
            buyer_ds.bought_list.push(data_id);
            seller_ds.bought_list.push(data_id);

            Self::deposit_event(RawEvent::Bought(buyer,seller,data_price));
            Ok(())
        }

         fn upload_to_space(origin,data_name: String,data: Vec<u8>) -> Result {
            Self::write_to_ipfs(data);
            Ok(())
        }

        fn download_from_space(origin,data_id: T::Hash) -> Result {

            //Self::read_from_ipfs(key);
            Ok(())
        }

        fn remove_from_space(origin,data_id: T::Hash) -> Result {

            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {

    fn write_to_ipfs(data: Vec<u8>) -> String {
        let mut ipfs_hash: String = "".to_string();
        let client: IpfsClient = IpfsClient::default();
        let req = client
            .add(Cursor::new(data)).map(|resp| ipfs_hash = resp.hash);

        #[cfg(feature = "hyper")]
        hyper::rt::run(req);
        ipfs_hash
    }

    fn read_from_ipfs(hash: String) -> String {
        let mut data: String = "".to_string();
        let client: IpfsClient = IpfsClient::default();
        let req = client.object_get(&hash).map(|resp| data=resp.data);
        data
    }

}
