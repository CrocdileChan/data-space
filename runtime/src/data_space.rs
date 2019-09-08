
use rstd::vec::Vec;
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::{As, Hash};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::Currency};
use system::ensure_signed;
use rstd::cmp;
//use parity_codec::alloc::vec::Vec;

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DataSpace<AccountId,Hash,Balance> {
    user_id: AccountId,
    user_name: Vec<u8>,
    // hashmap key is ipfs_hash
    data_to_price: Vec<Data<AccountId,Hash,Balance>>,
    bought_list: Vec<Hash>,
    sold_list: Vec<Hash>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Data<AccountId,Hash,Balance>{
    id: Hash,
    name: Vec<u8>,
    owner: AccountId,
    //ipfs_key: Vec<u8>,
    //content: Vec<u8>,
    price: Balance,
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
        Uploaded(AccountId,Hash),
        Removed(AccountId,Hash),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as DataStore {
        Owners get(owner_of): map T::AccountId => DataSpace<T::AccountId,T::Hash,T::Balance>;

        // data id => data struct
        DataInfo get(data): map T::Hash => Option<Data<T::AccountId,T::Hash,T::Balance>>;

        Nonce get(get_n): u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn register_account(origin, user_name: Vec<u8>) -> Result {
            let user_id = ensure_signed(origin)?;
            //let dp: Vec<Data<T::Hash,T::Balance>> = Vec::new();
            let new_data_space: DataSpace<T::AccountId,T::Hash,T::Balance> = DataSpace{
                user_id: user_id.clone(),
                user_name: user_name,
                data_to_price: Vec::new(),
                bought_list: Vec::new(),
                sold_list: Vec::new(),
            };
            Self::register(user_id.clone(),new_data_space);

            Ok(())
        }

        fn set_price(origin, data_id: T::Hash,new_price: T::Balance) -> Result{
            let user = ensure_signed(origin)?;

            Self::deposit_event(RawEvent::PriceSet(user,data_id,new_price));
            Ok(())
        }

        fn buy_data(origin, seller: T::AccountId, data_id: T::Hash, pay_price: T::Balance) -> Result {
            let buyer = ensure_signed(origin)?;
            ensure!(buyer != seller, "you can't buy your own data");

            if let Some(data) = Self::data(data_id){
                let data_price = data.price;
                ensure!(data_price <= pay_price, "the data you want to buy costs more than your pay");

                <balances::Module<T> as Currency<_>>::transfer(&buyer, &seller, data_price)?;

                let mut seller_ds = Self::owner_of(&seller);//.ok_or("no seller account you want")?;
                let mut buyer_ds = Self::owner_of(&buyer);//.or_not("no account registered to buy data")?;
                buyer_ds.bought_list.push(data_id);
                seller_ds.sold_list.push(data_id);

                Self::deposit_event(RawEvent::Bought(buyer,seller,data_id,data_price));
                Ok(())
            }else {
                Err(())
            };
            Ok(())
        }

         fn upload_to_space(origin,data_name: Vec<u8>,data: Vec<u8>,price: T::Balance) -> Result {
            let user = ensure_signed(origin)?;
            let nonce = Self::get_n();
            let random_hash = <system::Module<T>>::random_seed();
            let data_id = (random_hash,&user,nonce).using_encoded(<T as system::Trait>::Hashing::hash);
            let data: Data<T::AccountId,T::Hash,T::Balance> = Data{
                id: data_id,
                name: data_name,
                owner: user.clone(),
                price: price,
            };

            Self::upload(user,data_id,data);
            <Nonce<T>>::mutate(|n| *n += 1);
            Ok(())
        }

        fn download_from_space(origin,data_id: T::Hash) -> Result {
            if let Some(data) = Self::data(data_id){
                let user = ensure_signed(origin)?;
                let ds = Self::owner_of(user);
                if Self::contains_data(data_id,ds){
                    Ok(data)
                }else {
                    Err(())
                };
                Ok(())
            }else {
                Err(())
            };
            Ok(())

        }

        fn remove_from_space(origin,data_id: T::Hash) -> Result {
            if let Some(data) = Self::data(data_id){
                let user = ensure_signed(origin)?;
                Self::remove(user,data_id);
                Ok(())
            }else {
                Err(())
            };
            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {

    fn register(user_id: T::AccountId,new_data_space: DataSpace<T::AccountId,T::Hash,T::Balance>) {
        <Owners<T>>::insert(&user_id,new_data_space);
        Self::deposit_event(RawEvent::Registered(user_id));
    }

    fn upload(user_id: T::AccountId, data_id: T::Hash, data: Data<T::AccountId,T::Hash,T::Balance>) {
        <DataInfo<T>>::insert(data_id.clone(),data.clone());
        let mut ds = Self::owner_of(&user_id);
        ds.data_to_price.push(data);
        Self::deposit_event(RawEvent::Uploaded(user_id,data_id));
    }

    fn remove(user_id: T::AccountId,data_id: T::Hash) {
        <DataInfo<T>>::remove(data_id.clone());
        //remove from owners.
        Self::deposit_event(RawEvent::Removed(user_id,data_id));
    }

    fn contains_data(data_id: T::Hash, ds: DataSpace<T::AccountId,T::Hash,T::Balance>) -> bool{
        //if contains in data_to_price or bought_list
        for dp in &ds.data_to_price {
            if dp.id == data_id {
                return true;
            }
        };

        for bl in ds.bought_list {
            if bl == data_id {
                return true;
            }
        };

        false
    }
}
