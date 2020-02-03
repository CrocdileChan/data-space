use rstd::vec::Vec;
use parity_codec::{Decode, Encode};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::{Currency,LockIdentifier,LockableCurrency,WithdrawReasons}};
use runtime_primitives::traits::Bounded;
use system::ensure_signed;
//use parity_codec::alloc::vec::Vec;

pub trait Trait: system::Trait+balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
}

const BUY_LOCK: LockIdentifier = *b"buy_lock";

decl_event! {
    pub enum Event<T>
    where
    <T as system::Trait>::AccountId
    {
       Transfered(Vec<u8>),
       Confirmed(AccountId,usize),
       TippedOff(bool),
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct OrderForm<Balance> {
    id: usize,
    order_name: Vec<u8>,
    content: Vec<u8>,
    unit_price: Balance,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DataMetadata<AccountId>{
    data_name: Vec<u8>,
    to_company: AccountId,
    order_id: usize,
    hash_key: u64,
}


decl_storage! {
    trait Store for Module<T: Trait> as DataStore {

        pub Company get(get_order): map T::AccountId => Vec<OrderForm<T::Balance>>;

        People get(get_data): map T::AccountId => Vec<DataMetadata<T::AccountId>>;

        // where user data is actually stored
        Data get(get_content): map u64 => Vec<u8>;

        Nonce get(get_n): u64;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn buy_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't buy your own data");
            let is_existed = <Company<T>>::exists(&company);
            if is_existed {
                Self::transfer_data(company,person,order_id)
            }else {
                Err("no company")
            }
        }

        fn confirm_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't confirm to buy your data");
            T::Currency::remove_lock(BUY_LOCK,&person);
            Self::deposit_event(RawEvent::Confirmed(person,order_id));
            Ok(())
        }

        fn tip_off_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't tip-off yourself");
            if let Some(metadata) = Self::get_metadata(&person,&company,order_id){
                let person_data = Self::get_from_chain(metadata.hash_key);
                if let Some(order) = Self::get_orderform(&company, order_id){
                    let is_legal = Self::validate_data(person_data, order.content);
                    if is_legal {
                        // the company does evil, we need to lock the company's account or other ways for punishment.
                        T::Currency::set_lock(BUY_LOCK, &company, Bounded::max_value(), T::BlockNumber::max_value(),WithdrawReasons::all());
                        T::Currency::remove_lock(BUY_LOCK, &person);
                    }
                    // if person does evil, we need to keep locking the person's account or other ways for punishment.

                    Self::deposit_event(RawEvent::TippedOff(is_legal));
                }else {
                    return Err("no orderform");
                };
                Ok(())
            }else {
                Err("no data to tip-off")
            }

        }

        fn upload_order(origin, order_name: Vec<u8>, content: Vec<u8>, unit_price: T::Balance) -> Result{
            let company = ensure_signed(origin)?;
            let is_existed = <Company<T>>::exists(&company);
            if !is_existed {
                <Company<T>>::insert(company.clone(), Vec::new());
            }
            let mut order_list = Self::get_order(&company);
            let new_order: OrderForm<T::Balance> = OrderForm{
                id: order_list.len(),
                order_name: order_name,
                content: content,
                unit_price: unit_price,
            };
            order_list.push(new_order);
            Ok(())
        }

        fn upload_data(origin, data_name: Vec<u8>, data_content: Vec<u8>, to_company: T::AccountId, order_id: usize) -> Result {
            let person = ensure_signed(origin)?;
            let is_existed = <People<T>>::exists(&to_company);
            if !is_existed {
                <People<T>>::insert(person.clone(), Vec::new());
            }
            let mut metadata_list = Self::get_data(&person);
            let hash_key = Self::add_to_chain(data_content);
            let new_data: DataMetadata<T::AccountId> = DataMetadata{
                hash_key: hash_key,
                data_name: data_name,
                to_company: to_company,
                order_id: order_id,
            };
            metadata_list.push(new_data);
            Ok(())
        }

        fn update_data(origin, data_name: Vec<u8>, data_content: Vec<u8>, to_company: T::AccountId, order_id: usize) -> Result {
            let person = ensure_signed(origin)?;
            let is_existed = <People<T>>::exists(&to_company);
            if !is_existed {
                <People<T>>::insert(person.clone(), Vec::new());
                Err("no person")
            }else {
                let metadata_list = Self::get_data(&person);
                for mut metadata in metadata_list {
                    if metadata.to_company == to_company && metadata.order_id == order_id {
                        let hash_key = Self::add_to_chain(data_content);
                        metadata.hash_key = hash_key;
                        metadata.data_name = data_name;
                        break;
                    }
                };
                Ok(())
            }

        }

    }
}

//#[cfg_attr(feature = "std",derive(Debug, Serialize, Deserialize))]
//pub struct Resp {
//    pub name: Vec<u8>,
//    pub hash: Vec<u8>,
//    pub size: Vec<u8>,
//}

impl<T: Trait> Module<T> {

    fn transfer_data(company: T::AccountId, person: T::AccountId, order_id: usize) -> Result{
        let orders = Self::get_order(&company);

        let metadata_list = Self::get_data(&person);
        for metadata in metadata_list {
            if metadata.to_company == company && metadata.order_id == order_id {

                for order in &orders {
                    if order.id == order_id {
                        let data = Self::get_from_chain(metadata.hash_key);
                        let pay = order.unit_price;
                        <balances::Module<T> as Currency<_>>::transfer(&company, &person, pay)?;
                        T::Currency::set_lock(BUY_LOCK,&person, Bounded::max_value(), T::BlockNumber::max_value(),WithdrawReasons::all());
                        Self::deposit_event(RawEvent::Transfered(data));
                    }
                }

            }
        };

        Ok(())
    }

    fn add_to_chain(value: Vec<u8>) -> u64
    {
        let hash_key= Self::get_n();
        <Data<T>>::insert(&hash_key,value);
        <Nonce<T>>::mutate(|n| *n += 1);
        hash_key
    }

    fn get_from_chain(key: u64) -> Vec<u8> {
        Self::get_content(key)
    }

    // Brief Implementation:
    // just validate if data is empty and data equals order
    fn validate_data(data: Vec<u8>, order: Vec<u8>) -> bool {
        data != order && !data.is_empty()
    }

    fn get_metadata(person: &T::AccountId, company: &T::AccountId, order_id: usize) -> Option<DataMetadata<T::AccountId>> {
        let mut data_metadata: Option<DataMetadata<T::AccountId>> = None;
        let metadata_list = Self::get_data(person);
        for metadata in metadata_list {
            if &metadata.to_company == company && metadata.order_id == order_id {
                data_metadata = Some(metadata)
            }
        }
        data_metadata

    }

    fn get_orderform(company: &T::AccountId, order_id: usize) -> Option<OrderForm<T::Balance>> {
        let mut order_form: Option<OrderForm<T::Balance>> = None;
        let order_list = Self::get_order(company);
        for order in order_list {
            if order.id == order_id {
                order_form = Some(order)
            }
        }
        order_form
    }

}
