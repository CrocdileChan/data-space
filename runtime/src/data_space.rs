use rstd::vec::Vec;
use parity_codec::{Decode, Encode};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,traits::{Currency,LockIdentifier,LockableCurrency,WithdrawReasons}};
use runtime_primitives::traits::{Bounded, One};
use system::ensure_signed;

pub trait Trait: system::Trait+balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
}

pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

const COMPANY_LOCK: LockIdentifier = *b"cpn_lock";
const PERSON_LOCK: LockIdentifier = *b"psn_lock";

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

// Companies publish OrderForm for people to let them know what data they want.
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct OrderForm<Balance> {
    // order id is the index of Vec<OrderForm<T::Balance>>
    id: usize,
    order_name: Vec<u8>,
    content: Vec<u8>,
    unit_price: Balance,
}

// People upload data to make a deal with company.
// Datametadata is some metadata of what they upload.
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
        // store the order forms of every company
        pub Company get(get_order): map T::AccountId => Vec<OrderForm<T::Balance>>;
        // store the metadata of every people's data
        People get(get_data): map T::AccountId => Vec<DataMetadata<T::AccountId>>;
        // where people data is actually stored
        Data get(get_content): map u64 => Vec<u8>;
        // use Nonce to assign hash_key to people's data
        Nonce get(get_n): u64;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        // Companies can buy the people's data by calling this API.
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

        // When companies find the data is OK, they confirm data to unlock the people's account,
        // if people do evil (upload an illegal data), companies can call tip_off_data().
        fn confirm_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't confirm to buy your data");
            T::Currency::remove_lock(PERSON_LOCK,&person);
            Self::deposit_event(RawEvent::Confirmed(person,order_id));
            Ok(())
        }

        // When company finds that the people did not fill the data in the form as required, call tip_off_data(),
        // the chain will check the data.
        // If it is, the chain will punish people by keeping locking his account.
        // Otherwise, the chain will punish company by locking its account.
        // Normally, this API will not be called.
        fn tip_off_data(origin, person: T::AccountId, order_id: usize) -> Result {
            let company = ensure_signed(origin)?;
            ensure!(company != person, "you can't tip-off yourself");
            if let Some(metadata) = Self::get_metadata(&person,&company,order_id){
                let person_data = Self::get_from_chain(metadata.hash_key);
                if let Some(order) = Self::get_orderform(&company, order_id){
                    let is_legal = Self::validate_data(person_data, order.content);
                    if is_legal {
                        // the company does evil, we need to lock the company's account or other ways for punishment.
                        T::Currency::set_lock(COMPANY_LOCK, &company, Bounded::max_value(), <system::Module<T>>::block_number() + One::one() ,WithdrawReasons::all());
                        T::Currency::remove_lock(PERSON_LOCK, &person);
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

        // Company publishes its order form onto chain for every people to have a look.
        fn publish_order(origin, order_name: Vec<u8>, content: Vec<u8>, unit_price: T::Balance) -> Result{
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

        // People can choose to upload their own data onto the chain for order form which they are interested in.
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

        // People can update their own data when they find somethine changed.
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
                        T::Currency::set_lock(PERSON_LOCK, &person, pay.into(),  <system::Module<T>>::block_number() + One::one(), WithdrawReasons::all());
                        Self::deposit_event(RawEvent::Transfered(data));
                    }
                }

            }
        };

        Ok(())
    }

    // store data which people upload onto the chain.
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
