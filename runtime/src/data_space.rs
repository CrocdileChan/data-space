
use rstd::vec::Vec;
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::Hash;
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::Currency};
use system::ensure_signed;

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct UserInfo<AccountId,Hash> {
    user_id: AccountId,
    user_name: Vec<u8>,
    user_type: u16,
    own_data: Vec<Hash>,
    bought_list: Vec<Hash>,
    sold_list: Vec<Hash>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Data<AccountId,Hash,Balance>{
    id: Hash,
    name: Vec<u8>,
    owner: AccountId,
    content: Vec<u8>,
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
        Downloaded(Vec<u8>),
        Removed(AccountId,Hash),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as DataStore {
        Owners get(owner_of): map T::AccountId => UserInfo<T::AccountId,T::Hash>;

        // data id => data struct
        // TODO: the data should be stored into distributed database through substrate.
        DataInfo get(data): map T::Hash => Option<Data<T::AccountId,T::Hash,T::Balance>>;

        Nonce get(get_n): u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn register_account(origin, user_name: Vec<u8>, user_type: u16) -> Result {
            let user_id = ensure_signed(origin)?;
            let new_data_space: UserInfo<T::AccountId,T::Hash> = UserInfo{
                user_id: user_id.clone(),
                user_name: user_name,
                user_type: user_type,
                own_data: Vec::new(),
                bought_list: Vec::new(),
                sold_list: Vec::new(),
            };
            Self::register(user_id.clone(),new_data_space);

            Ok(())
        }

        fn set_price(origin, data_id: T::Hash,new_price: T::Balance) -> Result{
            let user = ensure_signed(origin)?;
            if <Owners<T>>::exists(&user){
                Err("no account")
            }else {
                if let Some(mut data) = Self::data(data_id){
                    data.price = new_price;
                    Self::deposit_event(RawEvent::PriceSet(user,data_id,new_price));
                    Ok(())
                }else {
                    Err("data not found")
                }
            }

        }

        fn buy_data(origin, seller: T::AccountId, data_id: T::Hash, pay_price: T::Balance) -> Result {
            let buyer = ensure_signed(origin)?;
            ensure!(buyer != seller, "you can't buy your own data");

            if let Some(data) = Self::data(data_id){
                let data_price = data.price;
                ensure!(data_price <= pay_price, "the data you want to buy costs more than your pay");

                <balances::Module<T> as Currency<_>>::transfer(&buyer, &seller, data_price)?;

                let mut seller_ds = Self::owner_of(&seller);
                let mut buyer_ds = Self::owner_of(&buyer);
                buyer_ds.bought_list.push(data_id);
                seller_ds.sold_list.push(data_id);

                Self::deposit_event(RawEvent::Bought(buyer,seller,data_id,data_price));
                Ok(())
            }else {
                Err("data not found")
            }

        }

         fn upload_to_space(origin,data_name: Vec<u8>,data: Vec<u8>,price: T::Balance) -> Result {
            let user = ensure_signed(origin)?;
            let nonce = Self::get_n();
            let random_hash = <system::Module<T>>::random_seed();
            let data_id = (random_hash,&user,nonce).using_encoded(<T as system::Trait>::Hashing::hash);
            let data_info: Data<T::AccountId,T::Hash,T::Balance> = Data{
                id: data_id,
                name: data_name,
                content: data,
                owner: user.clone(),
                price: price,
            };

            Self::upload(user,data_id,data_info);
            <Nonce<T>>::mutate(|n| *n += 1);
            Ok(())
        }

        fn download_from_space(origin,data_id: T::Hash) -> Result {
            if let Some(data) = Self::data(data_id){
                let user = ensure_signed(origin)?;
                let ds = Self::owner_of(user);
                if Self::contains_data(data_id,ds){
                    Self::deposit_event(RawEvent::Downloaded(data.content));
                    Ok(())
                }else {
                   Err("you don't own the data")
                }
            }else {
                   Err("data not found")
            }

        }

        fn remove_from_space(origin,data_id: T::Hash) -> Result {
            if let Some(_data) = Self::data(data_id){
                let user = ensure_signed(origin)?;
                Self::remove(user,data_id);
                Ok(())
            }else {
                Err("data not found")
            }
        }

    }
}

impl<T: Trait> Module<T> {

    fn register(user_id: T::AccountId,new_data_space: UserInfo<T::AccountId,T::Hash>) {
        <Owners<T>>::insert(&user_id,new_data_space);
        Self::deposit_event(RawEvent::Registered(user_id));
    }

    fn upload(user_id: T::AccountId, data_id: T::Hash, data: Data<T::AccountId,T::Hash,T::Balance>) {
        <DataInfo<T>>::insert(data_id.clone(),data.clone());
        let mut ds = Self::owner_of(&user_id);
        ds.own_data.push(data_id.clone());
        Self::deposit_event(RawEvent::Uploaded(user_id,data_id));
    }

    fn remove(user_id: T::AccountId,data_id: T::Hash) {
        <DataInfo<T>>::remove(data_id.clone());
        let mut ds: UserInfo<T::AccountId,T::Hash> = Self::owner_of(&user_id);
        ds.own_data.retain(|&x| x!=data_id);
        Self::deposit_event(RawEvent::Removed(user_id,data_id));
    }

    fn contains_data(data_id: T::Hash, ds: UserInfo<T::AccountId,T::Hash>) -> bool{
        //if contains in own_data or bought_list
        for od in ds.own_data {
            if od == data_id {
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
